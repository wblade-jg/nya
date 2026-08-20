use futures_util::StreamExt;
use reqwest::Client;
use std::env;
use std::path::Path;
use std::sync::LazyLock;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::io::BufWriter;

mod parser;

const REPOSITORIES_FILEPATH: &str = "sources";
const DEFAULT_CACHE_DIR: &str = "/etc/nya/";
static DOWNLOAD_PATH: LazyLock<String> = LazyLock::new(configure_download_path);

fn try_read_env_var(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| String::from(default.trim()))
}

fn generate_download_filename(prefix: &str, suffix: &str) -> String {
    format!("{}_{}", prefix, suffix)
}

fn configure_download_path() -> String {
    let cache_env = try_read_env_var("CACHE_DIR", DEFAULT_CACHE_DIR);

    let base_dir = Path::new(&cache_env);

    if !base_dir.exists() {
        std::fs::create_dir_all(&base_dir).expect("No se pudo crear el directorio de cache");
    }
    cache_env
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    match parser::read_repositories_from_file(REPOSITORIES_FILEPATH).await {
        Ok(repositories) => {
            let client = Client::new();
            futures_util::stream::iter(repositories.into_iter().map(|repository| {
                let client_clone = client.clone();
                let url = repository.inrelease_url().unwrap();
                let filename = generate_download_filename(
                    &repository.download_basename("_").unwrap(),
                    "InRelease",
                );
                async move {
                    if let Err(e) = download_file(&url, &filename, client_clone).await {
                        eprintln!("Error downloading {}: {}", url, e);
                    }
                    println!("Downloaded {}", url);
                }
            }))
            .buffer_unordered(10)
            .collect::<Vec<()>>()
            .await;
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }

    Ok(())
}

async fn download_file(
    url: &str,
    filename: &str,
    client: Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = client.get(url).send().await?.bytes_stream();

    let filename = Path::new(&*DOWNLOAD_PATH).join(filename);
    let file = File::create(filename).await?;

    let mut writer = BufWriter::new(file);
    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        writer.write_all(&bytes).await?;
    }
    writer.flush().await?;
    Ok(())
}
