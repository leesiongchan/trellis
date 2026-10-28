//! Validated roots for private, persistent knowledge data.
use crate::error::{AppError, ErrorCode};
use std::{
    collections::HashMap,
    ffi::OsString,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub data_dir: PathBuf,
    pub backup_dir: PathBuf,
    pub model_dir: PathBuf,
    pub asset_dir: PathBuf,
}

fn invalid() -> AppError {
    AppError::new(ErrorCode::InvalidConfig)
}

pub(super) fn validate_repository_roots(
    data: &Path,
    backup: &Path,
    repository: &Path,
) -> Result<(), AppError> {
    // The source checkout is a development guard, not a packaged runtime input.
    if let Ok(repository) = std::fs::canonicalize(repository)
        && (data.starts_with(&repository) || backup.starts_with(&repository))
    {
        return Err(invalid());
    }
    for path in [data, backup] {
        if path
            .ancestors()
            .any(|ancestor| ancestor.join(".git").exists())
        {
            return Err(invalid());
        }
    }
    Ok(())
}

/// Resolve existing ancestors, including symlinks, before comparing roots.
/// Reject dot traversal even when its intermediate directory does not exist.
pub(super) fn absolute(path: &Path) -> Result<PathBuf, AppError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err(invalid());
    }
    let mut ancestor = path;
    let mut remaining = Vec::<OsString>::new();
    loop {
        match std::fs::canonicalize(ancestor) {
            Ok(mut resolved) => {
                for part in remaining.into_iter().rev() {
                    resolved.push(part);
                }
                if resolved.parent().is_none() {
                    return Err(invalid());
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                remaining.push(ancestor.file_name().ok_or_else(invalid)?.to_owned());
                ancestor = ancestor.parent().ok_or_else(invalid)?;
            }
            Err(_) => return Err(invalid()),
        }
    }
}

impl StorageConfig {
    pub(super) fn validated(&self) -> Result<Self, AppError> {
        let data_dir = absolute(&self.data_dir)?;
        let backup_dir = absolute(&self.backup_dir)?;
        let model_dir = absolute(&self.model_dir)?;
        let asset_dir = absolute(&self.asset_dir)?;
        let roots = [&data_dir, &backup_dir, &model_dir, &asset_dir];
        for (index, a) in roots.iter().enumerate() {
            if roots[index + 1..]
                .iter()
                .any(|b| a.starts_with(b) || b.starts_with(a))
            {
                return Err(invalid());
            }
        }
        validate_repository_roots(
            &data_dir,
            &backup_dir,
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .ok_or_else(invalid)?,
        )?;
        Ok(Self {
            data_dir,
            backup_dir,
            model_dir,
            asset_dir,
        })
    }

    pub fn from_values(
        values: &HashMap<String, String>,
        executable: &Path,
    ) -> Result<Self, AppError> {
        let data_dir = absolute(Path::new(values.get("APP_DATA_DIR").ok_or_else(invalid)?))?;
        let executable_dir = executable.parent().ok_or_else(invalid)?;
        let backup_default = data_dir.parent().ok_or_else(invalid)?.join("backups");
        let model_default = executable_dir.join("models");
        let asset_default = executable_dir.join("public");
        let configured = |key: &str, fallback: &Path| {
            absolute(values.get(key).map(Path::new).unwrap_or(fallback))
        };
        let backup_dir = configured("APP_BACKUP_DIR", &backup_default)?;
        let model_dir = configured("APP_MODEL_DIR", &model_default)?;
        let asset_dir = configured("FRONTEND_DIR", &asset_default)?;
        let roots = [&data_dir, &backup_dir, &model_dir, &asset_dir];
        for (index, a) in roots.iter().enumerate() {
            if roots[index + 1..]
                .iter()
                .any(|b| a.starts_with(b) || b.starts_with(a))
            {
                return Err(invalid());
            }
        }
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(invalid)?;
        validate_repository_roots(&data_dir, &backup_dir, repository)?;
        Ok(Self {
            data_dir,
            backup_dir,
            model_dir,
            asset_dir,
        })
    }
}
