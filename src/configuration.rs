use crate::error::NyaError;
use std::{env, path::Path};

const DEFAULT_CACHE_DIR: &str = "/etc/nya";
const DEFAULT_REPOSITORIES_FILE: &str = "sources";

pub struct Configuration {
    pub repositories_filepath: String,
    pub download_path: String,
}

impl Configuration {
    pub fn from_env() -> Result<Self, NyaError> {
        let repositories_filepath = resolve_repositories_file(&try_read_env_var(
            "REPOSITORIES_FILE",
            DEFAULT_REPOSITORIES_FILE,
        ))?;
        let download_path = resolve_cache_dir(&try_read_env_var("CACHE_DIR", DEFAULT_CACHE_DIR))?;
        Ok(Configuration {
            repositories_filepath,
            download_path,
        })
    }
}

fn try_read_env_var(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_string())
}

pub(crate) fn resolve_cache_dir(value: &str) -> Result<String, NyaError> {
    let base_dir = Path::new(value);

    if !base_dir.is_dir() {
        std::fs::create_dir_all(base_dir)
            .map_err(|source| NyaError::io("crear el directorio de caché", value, source))?;
    }
    Ok(value.to_string())
}

pub(crate) fn resolve_repositories_file(value: &str) -> Result<String, NyaError> {
    if !Path::new(value).exists() {
        return Err(NyaError::MissingRepositoriesFile {
            path: value.to_string(),
        });
    }
    Ok(value.to_string())
}

pub(crate) fn to_debian_arch(arch: &str) -> Result<&'static str, NyaError> {
    match arch {
        "x86_64" => Ok("amd64"),
        "aarch64" => Ok("arm64"),
        other => Err(NyaError::UnsupportedArch(other.to_string())),
    }
}

pub fn architecture() -> Result<&'static str, NyaError> {
    to_debian_arch(std::env::consts::ARCH)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_debian_arch_maps_supported_architectures() {
        assert_eq!(to_debian_arch("x86_64").ok(), Some("amd64"));
        assert_eq!(to_debian_arch("aarch64").ok(), Some("arm64"));
    }

    #[test]
    fn to_debian_arch_rejects_unsupported_architecture() {
        let err = to_debian_arch("riscv64").unwrap_err();
        assert!(
            matches!(&err, NyaError::UnsupportedArch(arch) if arch == "riscv64"),
            "esperaba UnsupportedArch con la arquitectura original: {err:?}"
        );
    }

    #[test]
    fn resolve_repositories_file_missing_path_errors() {
        let err = resolve_repositories_file("/no/existe/sources").unwrap_err();
        assert!(
            matches!(&err, NyaError::MissingRepositoriesFile { path } if path == "/no/existe/sources"),
            "el error debe portar el path exacto: {err:?}"
        );
    }

    #[test]
    fn resolve_repositories_file_existing_path_ok() {
        let result = resolve_repositories_file("Cargo.toml");
        assert_eq!(result.unwrap(), "Cargo.toml");
    }

    #[test]
    fn resolve_cache_dir_creates_missing_directory() {
        let dir = std::env::temp_dir().join(format!("nya-cache-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        let result = resolve_cache_dir(dir.to_str().unwrap());
        assert!(
            result.is_ok(),
            "un directorio nuevo debe crearse: {result:?}"
        );
        assert!(dir.is_dir(), "el directorio debe existir tras resolver");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_cache_dir_over_existing_file_errors() {
        let file = std::env::temp_dir().join(format!("nya-cache-file-{}", std::process::id()));
        std::fs::write(&file, "x").unwrap();

        let err = resolve_cache_dir(file.to_str().unwrap()).unwrap_err();
        std::fs::remove_file(&file).ok();

        assert!(
            matches!(&err, NyaError::Io { path, .. } if path == file.to_str().unwrap()),
            "esperaba Io con el path del fichero que bloquea el directorio: {err:?}"
        );
    }
}
