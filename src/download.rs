use std::env;
use std::path::Path;
use std::sync::LazyLock;

use futures::stream::StreamExt;
use reqwest::Client;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};

pub const DEFAULT_CACHE_DIR: &str = "/etc/nya/";

static DOWNLOAD_PATH: LazyLock<String> = LazyLock::new(configure_download_path);

fn try_read_env_var(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| String::from(default.trim()))
}

fn configure_download_path() -> String {
    let cache_env = try_read_env_var("CACHE_DIR", DEFAULT_CACHE_DIR);

    let base_dir = Path::new(&cache_env);

    if !base_dir.exists() {
        std::fs::create_dir_all(&base_dir).expect("No se pudo crear el directorio de cache");
    }
    cache_env
}

pub async fn download_file(
    url: &str,
    filename: &str,
    client: Client,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut stream = client.get(url).send().await?.bytes_stream();

    let filepath = format!("{}/{}", &*DOWNLOAD_PATH, filename);
    let mut file = File::create(&filepath).await?;

    let mut writer = BufWriter::new(&mut file);
    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        writer.write_all(&bytes).await?;
    }
    writer.flush().await?;
    Ok(filepath)
}