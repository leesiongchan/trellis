//! Startup-verified assets are served from memory, never arbitrary filesystem paths.
use crate::{
    error::{AppError, ErrorCode},
    http::headers::{BASE_CSP, context_id},
};
use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Request, State},
    http::{HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use rustix::fs::{Mode, OFlags, open, openat};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::{Component, Path},
    sync::Arc,
};
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Manifest {
    version: u8,
    files: Vec<ManifestFile>,
    inline_script_hashes: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestFile {
    path: String,
    sha256: String,
    immutable: bool,
}
struct Asset {
    bytes: Bytes,
    mime: HeaderValue,
    etag: HeaderValue,
    immutable: bool,
}
pub struct StaticAssets {
    files: HashMap<String, Asset>,
    csp: HeaderValue,
}
impl StaticAssets {
    pub fn load(asset_dir: &Path) -> Result<Self, AppError> {
        let invalid = || AppError::new(ErrorCode::InvalidConfig);
        let manifest_path = asset_dir
            .parent()
            .ok_or_else(invalid)?
            .join("frontend-manifest.json");
        let manifest_fd = open(
            &manifest_path,
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::empty(),
        )
        .map_err(|_| invalid())?;
        let manifest: Manifest = serde_json::from_slice(
            &read_limited(File::from(manifest_fd), 4 * 1024 * 1024).map_err(|_| invalid())?,
        )
        .map_err(|_| invalid())?;
        if manifest.version != 1
            || manifest.files.len() > 4096
            || manifest.inline_script_hashes.len() > 32
        {
            return Err(invalid());
        }
        let root = File::from(
            open(
                asset_dir,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
                Mode::empty(),
            )
            .map_err(|_| invalid())?,
        );
        let mut files = HashMap::new();
        let mut total = 0usize;
        for entry in manifest.files {
            if !safe_path(&entry.path)
                || entry.immutable != entry.path.starts_with("_app/immutable/")
            {
                return Err(invalid());
            }
            let bytes = read_relative(&root, &entry.path).map_err(|_| invalid())?;
            total = total.checked_add(bytes.len()).ok_or_else(invalid)?;
            if total > 128 * 1024 * 1024 {
                return Err(invalid());
            }
            if hex::encode(Sha256::digest(&bytes)) != entry.sha256 {
                return Err(invalid());
            }
            let mime = HeaderValue::from_str(
                mime_guess::from_path(&entry.path)
                    .first_or_octet_stream()
                    .as_ref(),
            )
            .map_err(|_| invalid())?;
            let etag =
                HeaderValue::from_str(&format!("\"{}\"", entry.sha256)).map_err(|_| invalid())?;
            if files
                .insert(
                    entry.path,
                    Asset {
                        bytes: Bytes::from(bytes),
                        mime,
                        etag,
                        immutable: entry.immutable,
                    },
                )
                .is_some()
            {
                return Err(invalid());
            }
        }
        if !files.contains_key("index.html") {
            return Err(invalid());
        }
        let mut script_policy = "script-src 'self'".to_owned();
        for hash in manifest.inline_script_hashes {
            let bytes = hash
                .strip_prefix("sha256-")
                .and_then(|value| STANDARD.decode(value).ok())
                .ok_or_else(invalid)?;
            if bytes.len() != 32 || hash != format!("sha256-{}", STANDARD.encode(bytes)) {
                return Err(invalid());
            }
            script_policy.push_str(&format!(" '{hash}'"));
        }
        let csp = HeaderValue::from_str(&BASE_CSP.replacen("script-src 'self'", &script_policy, 1))
            .map_err(|_| invalid())?;
        Ok(Self { files, csp })
    }
}
fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && path
            .split('/')
            .all(|p| !p.is_empty() && !p.starts_with('.'))
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        && matches!(
            Path::new(path).extension().and_then(|s| s.to_str()),
            Some(
                "html"
                    | "js"
                    | "css"
                    | "json"
                    | "svg"
                    | "ico"
                    | "png"
                    | "jpg"
                    | "jpeg"
                    | "webp"
                    | "avif"
                    | "gif"
                    | "woff"
                    | "woff2"
                    | "ttf"
                    | "eot"
                    | "txt"
            )
        )
}
fn read_limited(file: File, max: u64) -> std::io::Result<Vec<u8>> {
    if !file.metadata()?.is_file() || file.metadata()?.len() > max {
        return Err(std::io::Error::other("invalid asset"));
    }
    let mut bytes = Vec::new();
    file.take(max + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max {
        return Err(std::io::Error::other("oversized asset"));
    }
    Ok(bytes)
}
fn read_relative(root: &File, path: &str) -> std::io::Result<Vec<u8>> {
    let mut parent = root.try_clone()?;
    let parts: Vec<_> = path.split('/').collect();
    for (index, part) in parts.iter().enumerate() {
        let last = index == parts.len() - 1;
        let flags = OFlags::RDONLY
            | OFlags::CLOEXEC
            | OFlags::NOFOLLOW
            | if last {
                OFlags::empty()
            } else {
                OFlags::DIRECTORY
            };
        let file = File::from(openat(&parent, *part, flags, Mode::empty())?);
        if last {
            return read_limited(file, 32 * 1024 * 1024);
        }
        parent = file;
    }
    Err(std::io::Error::other("invalid asset path"))
}
pub fn routes(assets: Arc<StaticAssets>) -> Router {
    Router::new().fallback(serve).with_state(assets)
}
async fn serve(State(assets): State<Arc<StaticAssets>>, request: Request) -> Response {
    let id = context_id(&request);
    if request.method() != Method::GET && request.method() != Method::HEAD {
        let mut response = AppError::new(ErrorCode::MethodNotAllowed).response(id);
        response
            .headers_mut()
            .insert("allow", HeaderValue::from_static("GET, HEAD"));
        return response;
    }
    let path = request.uri().path();
    let name = if path == "/" {
        "index.html"
    } else {
        path.strip_prefix('/').unwrap_or(path)
    };
    let Some(asset) = assets.files.get(name) else {
        return AppError::new(ErrorCode::NotFound).response(id);
    };
    let unchanged = request.headers().get("if-none-match") == Some(&asset.etag);
    let mut response = if unchanged {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        let mut response = Body::from(if request.method() == Method::HEAD {
            Bytes::new()
        } else {
            asset.bytes.clone()
        })
        .into_response();
        if let Ok(length) = HeaderValue::from_str(&asset.bytes.len().to_string()) {
            response.headers_mut().insert("content-length", length);
        }
        response
    };
    let headers = response.headers_mut();
    headers.insert("content-type", asset.mime.clone());
    headers.insert("etag", asset.etag.clone());
    headers.insert("content-security-policy", assets.csp.clone());
    headers.insert(
        "cache-control",
        HeaderValue::from_static(if asset.immutable {
            "public, max-age=31536000, immutable"
        } else {
            "no-cache"
        }),
    );
    response
}
