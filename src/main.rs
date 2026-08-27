use futures::stream::{self, StreamExt};
use nya::{download, parser, signature};
use reqwest::Client;
use tokio::task;

const REPOSITORIES_FILEPATH: &str = "sources";

fn generate_download_filename(prefix: &str, suffix: &str) -> String {
    format!("{}_{}", prefix, suffix)
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
                    let result = download::download_file(
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