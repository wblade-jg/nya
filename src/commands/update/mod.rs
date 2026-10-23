use crate::configuration::{Configuration, architecture};
use crate::types::{DownloadedInRelease, PackageFileInfo, Repository};
use crate::{download, integrity, package_info, parser, url};
use futures::stream::{self, StreamExt};
use reqwest::Client;
use std::path::Path;
use std::time::Duration;
use tokio::task;

fn repo_label(repository: &Repository) -> String {
    let primera_uri = repository
        .urls()
        .next()
        .map(String::as_str)
        .unwrap_or("repositorio sin URIs");
    match repository.suite() {
        Some(suite) => format!("{primera_uri} ({suite})"),
        None => primera_uri.to_string(),
    }
}

fn mirror_error(what: &str, causas: &[String]) -> String {
    if causas.is_empty() {
        return format!("No hay mirrors de {what} que probar");
    }
    let lista = causas
        .iter()
        .map(|causa| format!("    - {causa}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{what} no descargable de ningún mirror:\n{lista}")
}

async fn execute_pipeline(
    repositories: Vec<Repository>,
    config: &Configuration,
) -> Result<(), Box<dyn std::error::Error>> {
    let total = repositories.len();
    if total == 0 {
        println!("==> Sin repositorios, nada que hacer");
        return Ok(());
    }
    let plural = if total == 1 {
        "repositorio"
    } else {
        "repositorios"
    };
    println!("==> {total} {plural} en sources");

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()?;

    let (ok, failures) = stream::iter(repositories.into_iter().map(|repository| {
        let label = repo_label(&repository);
        let download = download_inrelease(repository, client.clone(), config);
        async move {
            let result = download.await.map_err(|e| e.to_string());
            (label, result)
        }
    }))
    .buffer_unordered(10)
    .map(|(label, result)| {
        let client = client.clone();
        async move {
            let result: Result<(), String> = match result {
                Err(e) => Err(e),
                Ok(downloaded) => {
                    match task::spawn_blocking(move || process_inrelease(downloaded)).await {
                        Err(e) => Err(format!("Error en spawn_blocking: {}", e)),
                        Ok(Err(e)) => Err(e.to_string()),
                        Ok(Ok((downloaded, package_info))) => {
                            println!("  ✓ InRelease verificado");
                            let resultado = download_packages_file(
                                downloaded.repository,
                                package_info,
                                config,
                                client,
                            )
                            .await;
                            if resultado.is_ok() {
                                println!("  ✓ Packages verificado (SHA256)");
                            }
                            resultado.map_err(|e| e.to_string())
                        }
                    }
                }
            };
            (label, result)
        }
    })
    .buffer_unordered(10)
    .map(|(label, result)| {
        result.map_err(|e| {
            let mensaje = format!("{label}: {e}");
            eprintln!("  ✗ {mensaje}");
            mensaje
        })
    })
    .fold((0usize, 0usize), |(ok, fail), result| async move {
        match result {
            Ok(()) => (ok + 1, fail),
            Err(_) => (ok, fail + 1),
        }
    })
    .await;

    if failures > 0 {
        return Err(format!(
            "{} de {} repositorios fallaron durante la actualización",
            failures, total
        )
        .into());
    }

    println!("==> Hecho: {ok}/{total} correcto");
    Ok(())
}

pub async fn update(config: &Configuration) -> Result<(), Box<dyn std::error::Error>> {
    let repositories = parser::read_repositories_from_file(&config.repositories_filepath).await?;
    execute_pipeline(repositories, config).await
}

async fn download_inrelease(
    repository: Repository,
    client: Client,
    config: &Configuration,
) -> Result<DownloadedInRelease, Box<dyn std::error::Error>> {
    let suite = repository
        .suite()
        .ok_or("No se encontró suite en el repositorio")?;
    let urls: Vec<_> = repository.urls().cloned().collect();
    let mut causas: Vec<String> = Vec::new();

    for url in urls {
        let download_url = format!("{}/dists/{}/InRelease", url, suite);
        let Some(filename) = url::format_url("_", &download_url) else {
            causas.push(format!("{download_url}: URL no válida"));
            continue;
        };

        match download::download_file(
            &download_url,
            &filename,
            &config.download_path,
            client.clone(),
        )
        .await
        {
            Ok(downloaded_filepath) => {
                return Ok(DownloadedInRelease::new(downloaded_filepath, repository));
            }
            Err(e) => causas.push(format!("{download_url}: {e}")),
        }
    }

    Err(mirror_error("InRelease", &causas).into())
}

fn process_inrelease(
    downloaded: DownloadedInRelease,
) -> Result<(DownloadedInRelease, PackageFileInfo), Box<dyn std::error::Error + Send + Sync>> {
    let content = integrity::validate_signature_file(
        downloaded.inrelease_path(),
        downloaded.signature_path(),
    )?;

    let package_info = package_info::get_package_file_info(
        &content,
        &downloaded.target_package_path(architecture())?,
    )?;

    Ok((downloaded, package_info))
}

async fn download_packages_file(
    repository: Repository,
    package_info: PackageFileInfo,
    config: &Configuration,
    client: Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let suite = repository
        .suite()
        .ok_or("No se encontró suite en el repositorio")?;
    let mut causas: Vec<String> = Vec::new();

    for base_url in repository.urls() {
        let download_url = format!("{}/dists/{}/{}", base_url, suite, package_info.path());
        let Some(filename) = url::format_url("_", &download_url) else {
            causas.push(format!("{download_url}: URL no válida"));
            continue;
        };
        let file_path = format!("{}/{}", config.download_path, filename);

        if Path::new(&file_path).exists() {
            let hash_result = task::spawn_blocking({
                let path = file_path.clone();
                move || integrity::calculate_file_hash(&path)
            })
            .await;

            match hash_result {
                Ok(Ok(existing_hash)) if existing_hash == package_info.hash() => {
                    println!(
                        "El fichero Packages ya existe con el hash correcto, se omite la descarga: {}",
                        file_path
                    );
                    return Ok(());
                }
                Ok(Ok(_)) => {
                    println!(
                        "El fichero Packages existe pero el hash no coincide, volviendo a descargar..."
                    );
                }
                Ok(Err(e)) => {
                    causas.push(format!("{download_url}: no se pudo calcular el hash: {e}"));
                }
                Err(e) => {
                    causas.push(format!("{download_url}: error en spawn_blocking: {e}"));
                }
            }
        }

        let downloaded_filepath = match download::download_file(
            &download_url,
            &filename,
            &config.download_path,
            client.clone(),
        )
        .await
        {
            Ok(filepath) => filepath,
            Err(e) => {
                causas.push(format!("{download_url}: {e}"));
                continue;
            }
        };

        let hash_result = task::spawn_blocking({
            let path = downloaded_filepath.clone();
            move || integrity::calculate_file_hash(&path)
        })
        .await;

        let downloaded_hash = match hash_result {
            Ok(Ok(hash)) => hash,
            Ok(Err(e)) => {
                causas.push(format!("{download_url}: no se pudo calcular el hash: {e}"));
                continue;
            }
            Err(e) => {
                causas.push(format!("{download_url}: error en spawn_blocking: {e}"));
                continue;
            }
        };

        if downloaded_hash != package_info.hash() {
            causas.push(format!(
                "{download_url}: el hash no coincide (esperado {}, obtenido {downloaded_hash})",
                package_info.hash()
            ));
            if let Err(e) = std::fs::remove_file(&downloaded_filepath) {
                eprintln!(
                    "No se pudo eliminar el fichero corrupto {}: {}",
                    downloaded_filepath, e
                );
            }
            continue;
        }

        return Ok(());
    }

    Err(mirror_error("Packages", &causas).into())
}
