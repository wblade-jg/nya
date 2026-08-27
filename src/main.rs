use futures::stream::{self, StreamExt};
use reqwest::Client;
use std::env;
use std::path::Path;
use std::sync::LazyLock;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::io::BufWriter;
use tokio::task;

mod parser;
mod signature;

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
            let pipeline = stream::iter(repositories.into_iter().map(|repository| {
                let client_clone = client.clone();
                async move {
                    let filename = generate_download_filename(
                        &repository.download_basename("_").unwrap(),
                        "InRelease",
                    );
                    let result = download_file(
                        &repository.inrelease_url().unwrap(),
                        &filename,
                        client_clone,
                    )
                    .await;

                    match result {
                        Ok(downloaded_filename) => {
                            let signature_filename = repository.signed_by().unwrap();
                            println!("Descargado: {}", downloaded_filename);
                            Some((downloaded_filename, signature_filename))
                        }
                        Err(e) => {
                            eprintln!("Error en la descarga: {}", e);
                            None
                        }
                    }
                }
            }))
            .buffer_unordered(10)
            .filter_map(|result| async move { result })
            .map(|(downloaded_filename, signature_filename)| async move {
                task::spawn_blocking(move || {
                    let result = signature::validate_signature_file(
                        &downloaded_filename,
                        &signature_filename,
                    );
                    match result {
                        Ok(message) => println!("{}", message),
                        Err(e) => eprintln!("Error verificando la firma: {}", e),
                    }
                })
                .await
            })
            .buffer_unordered(10);

            pipeline.collect::<Vec<_>>().await;
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