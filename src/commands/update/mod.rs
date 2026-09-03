use crate::configuration::Configuration ;
use crate::types::{DownloadedInRelease, PackageFileInfo, Repository};
use crate::{download, package_info, parser, signature, url};
use futures::stream::{self, StreamExt};
use reqwest::Client;
use sha2::{Digest, Sha256};
use tokio::task;

fn calculate_file_hash(file_path: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    use std::fs::File;
    use std::io::Read;
    
    let mut file = File::open(file_path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    
    let result = hasher.finalize();
    let hex_string = result.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    
    Ok(hex_string)
}

async fn execute_pipeline(repositories: Vec<Repository>, config: &Configuration) {
    let client = Client::new();

    let pipeline =
        stream::iter(
            repositories
                .into_iter()
                .map(|repository| download_inrelease(repository, client.clone(), config)),
        )
        .buffer_unordered(10)
        .filter_map(|result| async move { result.ok() })
        .map(|downloaded| async move {
            task::spawn_blocking(move || process_inrelease(downloaded)).await
        })
        .buffer_unordered(10)
        .filter_map(|result| async move { result.map_err(|e| eprintln!("{}", e)).ok() });

    pipeline.collect::<Vec<_>>().await;
}

pub async fn update(config: &Configuration) {
    match parser::read_repositories_from_file(&config.repositories_filepath).await {
        Ok(repositories) => {
            execute_pipeline(repositories, config).await;
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}

async fn download_inrelease(
    repository: Repository,
    client: Client,
    config: &Configuration
) -> Result<DownloadedInRelease, Box<dyn std::error::Error>> {
    let download_url = repository
        .inrelease_path()
        .ok_or("No se pudo construir la ruta del InRelease")?;

    let filename = url::format_url("_", &download_url)
        .ok_or("No se pudo formatear la URL del InRelease")?;
    
    let inrelease_path = download::download_file(&download_url, &filename, &config.download_path, client)
        .await
        .map_err(|e| {
            eprintln!("Error en la descarga: {}", e);
            e
        })?;

    println!("Descargado: {}", inrelease_path);
    DownloadedInRelease::from_repository(&repository, inrelease_path)
}

fn process_inrelease(
    downloaded: DownloadedInRelease,
) -> Result<PackageFileInfo, Box<dyn std::error::Error + Send + Sync>> {
    let content = signature::validate_signature_file(
        downloaded.inrelease_path(),
        downloaded.signature_path(),
    )?;

    package_info::get_package_file_info(&content, downloaded.target_package_path())
}
