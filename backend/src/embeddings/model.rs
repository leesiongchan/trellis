use super::*;
use crate::storage::records::canonical_hash;
use fastembed::{
    InitOptionsUserDefined, Pooling, QuantizationMode, TextEmbedding, TokenizerFiles,
    UserDefinedEmbeddingModel,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
};
use tokenizers::Tokenizer;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelManifest {
    pub model_id: String,
    pub revision: String,
    pub dimensions: usize,
    pub max_tokens: usize,
    pub artifact_hashes: BTreeMap<String, String>,
    pub preprocessing_version: String,
    #[serde(skip)]
    pub(crate) tokenizer: Option<Arc<Tokenizer>>,
}
impl ModelManifest {
    pub fn pinned() -> Result<Self, EmbeddingError> {
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/library/model-manifest.json"
        ))
        .map_err(|_| EmbeddingError::CorruptModel)
    }
    pub fn version(&self) -> String {
        canonical_hash(
            &serde_json::json!({"modelId":self.model_id,"revision":self.revision,"dimensions":self.dimensions,"maxTokens":self.max_tokens,"artifactHashes":self.artifact_hashes,"preprocessingVersion":self.preprocessing_version}),
        )
    }
    pub fn with_tokenizer(&self, root: &Path) -> Result<Self, EmbeddingError> {
        let bytes = verified_file(root, "tokenizer.json", self)?;
        let mut tokenizer =
            Tokenizer::from_bytes(bytes).map_err(|_| EmbeddingError::CorruptModel)?;
        tokenizer
            .with_truncation(None)
            .map_err(|_| EmbeddingError::CorruptModel)?;
        tokenizer.with_padding(None);
        let mut manifest = self.clone();
        manifest.tokenizer = Some(Arc::new(tokenizer));
        Ok(manifest)
    }
    pub fn token_count(&self, text: &str, special: bool) -> Result<usize, EmbeddingError> {
        Ok(self.tokens(text, special)?.len())
    }
    pub(crate) fn tokens(
        &self,
        text: &str,
        special: bool,
    ) -> Result<tokenizers::Encoding, EmbeddingError> {
        self.tokenizer
            .as_ref()
            .ok_or(EmbeddingError::MissingModel)?
            .encode(text, special)
            .map_err(|_| EmbeddingError::InvalidInput)
    }
    pub fn truncate_input(&self, text: &str, max_tokens: usize) -> Result<String, EmbeddingError> {
        if max_tokens == 0 {
            return Ok(String::new());
        }
        let encoded = self.tokens(text, false)?;
        if encoded.len() <= max_tokens {
            return Ok(text.into());
        }
        let end = encoded
            .get_offsets()
            .get(max_tokens.saturating_sub(1))
            .map(|(_, end)| *end)
            .unwrap_or(0);
        Ok(text.get(..end).ok_or(EmbeddingError::InvalidInput)?.into())
    }
}
fn verified_file(
    root: &Path,
    name: &str,
    manifest: &ModelManifest,
) -> Result<Vec<u8>, EmbeddingError> {
    if !root.is_absolute()
        || !matches!(
            name,
            "model_optimized.onnx"
                | "tokenizer.json"
                | "tokenizer_config.json"
                | "special_tokens_map.json"
                | "config.json"
        )
    {
        return Err(EmbeddingError::CorruptModel);
    }
    let path = root.join(name);
    let meta = std::fs::symlink_metadata(&path).map_err(|_| EmbeddingError::MissingModel)?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 128 * 1024 * 1024 {
        return Err(EmbeddingError::CorruptModel);
    }
    let bytes = std::fs::read(path).map_err(|_| EmbeddingError::MissingModel)?;
    let actual = format!("{:x}", Sha256::digest(&bytes));
    if manifest.artifact_hashes.get(name) != Some(&actual) {
        return Err(EmbeddingError::CorruptModel);
    }
    Ok(bytes)
}
pub struct LocalEmbedder {
    manifest: ModelManifest,
    engine: Mutex<TextEmbedding>,
}
impl LocalEmbedder {
    pub fn load(root: &Path, manifest: &ModelManifest) -> Result<Self, EmbeddingError> {
        if manifest.version() != ModelManifest::pinned()?.version() {
            return Err(EmbeddingError::CorruptModel);
        }
        #[cfg(target_arch = "x86_64")]
        if !std::is_x86_feature_detected!("avx2") {
            return Err(EmbeddingError::RuntimeUnavailable);
        }
        // Bring-your-own files only: the hf-hub feature is disabled at compile time.
        let onnx = verified_file(root, "model_optimized.onnx", manifest)?;
        let tokenizer_files = TokenizerFiles {
            tokenizer_file: verified_file(root, "tokenizer.json", manifest)?,
            config_file: verified_file(root, "config.json", manifest)?,
            special_tokens_map_file: verified_file(root, "special_tokens_map.json", manifest)?,
            tokenizer_config_file: verified_file(root, "tokenizer_config.json", manifest)?,
        };
        let model = UserDefinedEmbeddingModel::new(onnx, tokenizer_files)
            .with_pooling(Pooling::Cls)
            .with_quantization(QuantizationMode::Static);
        let engine = TextEmbedding::try_new_from_user_defined(
            model,
            InitOptionsUserDefined::new()
                .with_auto_device(ort::session::builder::AutoDevicePolicy::PreferCPU)
                .with_max_length(manifest.max_tokens)
                .with_intra_threads(2),
        )
        .map_err(|_| EmbeddingError::RuntimeUnavailable)?;
        Ok(Self {
            manifest: manifest.with_tokenizer(root)?,
            engine: Mutex::new(engine),
        })
    }
}
impl Embedder for LocalEmbedder {
    fn manifest(&self) -> &ModelManifest {
        &self.manifest
    }
    fn embed(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        if inputs.is_empty() {
            return Ok(vec![]);
        }
        if inputs.len() > 32 {
            return Err(EmbeddingError::InvalidInput);
        }
        // The public spike used raw text for both query and document channels.
        // CLS pooling and L2 normalization are pinned with that preprocessing version.
        let texts = inputs
            .iter()
            .map(|input| {
                let text = match input.kind {
                    InputKind::Query | InputKind::Document => input.text.as_str(),
                };
                if self.manifest.token_count(text, true)? > self.manifest.max_tokens {
                    return Err(EmbeddingError::InvalidInput);
                }
                Ok(text)
            })
            .collect::<Result<Vec<_>, EmbeddingError>>()?;
        let vectors = self
            .engine
            .lock()
            .map_err(|_| EmbeddingError::RuntimeUnavailable)?
            .embed(texts, Some(32))
            .map_err(|_| EmbeddingError::RuntimeUnavailable)?;
        if vectors.len() != inputs.len() {
            return Err(EmbeddingError::InvalidInput);
        }
        vectors
            .into_iter()
            .map(|v| {
                SearchVector::new(self.manifest.version(), v, self.manifest.dimensions)
                    .map(|v| v.values)
            })
            .collect()
    }
}
