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

pub fn architecture() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => panic!("Arquitectura no soportada: {other}"),
    }
}
