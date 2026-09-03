use crate::configuration::{Configuration, architecture};
use crate::types::{DownloadedInRelease, PackageFileInfo, Repository};
use crate::{download, package_info, parser, integrity, url};
use futures::stream::{self, StreamExt};
use reqwest::Client;
use std::path::Path;
use tokio::task;

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
        .map(|downloaded| {
            let client = client.clone();
            async move {
                let result = task::spawn_blocking(move || process_inrelease(downloaded))
                    .await
                    .map_err(|e| format!("Error en spawn_blocking: {}", e))?;
                
                match result {
                    Ok((downloaded_inrelease, package_info)) => {
                        download_packages_file(
                            downloaded_inrelease.repository,
                            package_info,
                            config,
                            client,
                        )
                        .await
                    }
                    Err(e) => Err(format!("Error procesando InRelease: {}", e).into()),
                }
            }
        })
        .buffer_unordered(10)
        .filter_map(|result: Result<(), Box<dyn std::error::Error>>| async move {
            if let Err(e) = &result {
                eprintln!("Error en el pipeline: {}", e);
            }
            result.ok()
        });

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
    config: &Configuration,
) -> Result<DownloadedInRelease, Box<dyn std::error::Error>> {
    let suite = repository.suite().ok_or("No se encontró suite en el repositorio")?;
    let urls: Vec<_> = repository.urls().cloned().collect();
    
    for url in urls {
        let download_url = format!("{}/dists/{}/InRelease", url, suite);
        let filename = url::format_url("_", &download_url)
            .ok_or("No se pudo formatear la URL del InRelease")?;
        
        if let Ok(downloaded_filepath) = download::download_file(
            &download_url,
            &filename,
            &config.download_path,
            client.clone(),
        )
        .await
        {
            println!("Descargado InRelease: {}", downloaded_filepath);
            return Ok(DownloadedInRelease::new(downloaded_filepath, repository));
        }
    }
    
    Err("No se pudo descargar el InRelease de ningún mirror".into())
}

fn process_inrelease(
    downloaded: DownloadedInRelease,
) -> Result<(DownloadedInRelease, PackageFileInfo), Box<dyn std::error::Error + Send + Sync>> {
    let content = integrity::validate_signature_file(
        downloaded.inrelease_path(),
        downloaded.signature_path(),
    )?;

    let package_info = package_info::get_package_file_info(&content, &downloaded.target_package_path(architecture()))?;
    
    Ok((downloaded, package_info))
}

async fn download_packages_file(
    repository: Repository,
    package_info: PackageFileInfo,
    config: &Configuration,
    client: Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut urls = repository.urls();
    let suite = repository.suite().ok_or("No se encontró suite en el repositorio")?;
    
    while let Some(base_url) = urls.next() {
        let download_url = format!("{}/dists/{}/{}", base_url, suite, package_info.path());
        
        let filename = url::format_url("_", &download_url)
            .ok_or("No se pudo formatear la URL del paquete")?;
        
        let file_path = format!("{}/{}", &config.download_path, filename);
        
        if Path::new(&file_path).exists() {
            if let Ok(hash_result) = task::spawn_blocking({
                let path = file_path.clone();
                move || integrity::calculate_file_hash(&path)
            }).await {
                if let Ok(existing_hash) = hash_result {
                    if existing_hash == package_info.hash() {
                        println!("Packages file already exists with correct hash, skipping download: {}", file_path);
                        return Ok(());
                    } else {
                        println!("Packages file exists but hash mismatch, re-downloading...");
                    }
                }
            }
        }
        
        if let Ok(downloaded_filepath) = download::download_file(
            &download_url,
            &filename,
            &config.download_path,
            client.clone(),
        )
        .await
        {
            if let Ok(hash_result) = task::spawn_blocking({
                let path = downloaded_filepath.clone();
                move || integrity::calculate_file_hash(&path)
            }).await {
                match hash_result {
                    Ok(downloaded_hash) => {
                        if downloaded_hash != package_info.hash() {
                            eprintln!("Hash mismatch after download. Expected: {}, Got: {}", 
                                package_info.hash(), downloaded_hash);
                            continue; 
                        }
                        
                        println!("Descargado y verificado Packages: {}", downloaded_filepath);
                        return Ok(());
                    }
                    Err(e) => {
                        eprintln!("Error calculando hash: {}", e);
                        continue;
                    }
                }
            }
        }
    }
    
    Err("No se pudo descargar el archivo Packages de ningún mirror".into())
}
