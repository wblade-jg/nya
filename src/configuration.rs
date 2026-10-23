use std::{env, path::Path};

const DEFAULT_CACHE_DIR: &str = "/etc/nya";
const DEFAULT_REPOSITORIES_FILE: &str = "sources";

pub struct Configuration {
    pub repositories_filepath: String,
    pub download_path: String,
}

impl Configuration {
    pub fn from_env() -> Self {
        Configuration {
            repositories_filepath: configure_repositories_file(),
            download_path: configure_download_path(),
        }
    }
}

fn try_read_env_var(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| String::from(default.trim()))
}

fn configure_download_path() -> String {
    let cache_env = try_read_env_var("CACHE_DIR", DEFAULT_CACHE_DIR);

    let base_dir = Path::new(&cache_env);

    if !base_dir.exists() {
        std::fs::create_dir_all(base_dir).expect("No se pudo crear el directorio de cache");
    }
    cache_env
}

fn configure_repositories_file() -> String {
    let repositories_file = try_read_env_var("REPOSITORIES_FILE", DEFAULT_REPOSITORIES_FILE);

    let base_dir = Path::new(&repositories_file);

    if !base_dir.exists() {
        panic!("No se pudo encontrar el archivo de repositorios");
    }
    repositories_file
}

pub(crate) fn to_debian_arch(arch: &str) -> Result<&'static str, String> {
    match arch {
        "x86_64" => Ok("amd64"),
        "aarch64" => Ok("arm64"),
        other => Err(format!("Arquitectura no soportada: {other}")),
    }
}

pub fn architecture() -> Result<&'static str, String> {
    to_debian_arch(std::env::consts::ARCH)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_debian_arch_maps_supported_architectures() {
        assert_eq!(to_debian_arch("x86_64"), Ok("amd64"));
        assert_eq!(to_debian_arch("aarch64"), Ok("arm64"));
    }

    #[test]
    fn to_debian_arch_rejects_unsupported_architecture_without_panicking() {
        let result = to_debian_arch("riscv64");
        assert!(
            result.is_err(),
            "una arquitectura no soportada debe devolver Err, no panic"
        );
        assert!(result.unwrap_err().contains("riscv64"));
    }
}
