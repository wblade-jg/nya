use std::sync::LazyLock;
use std::{env, path::Path};

pub static REPOSITORIES_FILEPATH: LazyLock<String> = LazyLock::new(configure_repositories_file);
pub static DOWNLOAD_PATH: LazyLock<String> = LazyLock::new(configure_download_path);
pub static ARCHITECTURE: LazyLock<&'static str> = LazyLock::new(architecture);
const DEFAULT_CACHE_DIR: &str = "/etc/nya";
const DEFAULT_REPOSITORIES_FILE: &str = "sources";

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

fn architecture() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => panic!("Arquitectura no soportada: {other}"),
    }
}
