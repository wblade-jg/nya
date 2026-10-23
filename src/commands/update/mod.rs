use crate::configuration::{Configuration, architecture};
use crate::types::{DownloadedInRelease, PackageFileInfo, Repository};
use crate::{download, integrity, package_info, parser, url};
use futures::stream::{self, StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
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

fn say(multi: &MultiProgress, linea: impl AsRef<str>) {
    if multi.is_hidden() {
        println!("{}", linea.as_ref());
    } else {
        let _ = multi.println(linea.as_ref());
    }
}

fn fail(multi: &MultiProgress, linea: impl AsRef<str>) {
    let linea = linea.as_ref().to_string();
    multi.suspend(move || eprintln!("{linea}"));
}

fn spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner:.green} {msg}").unwrap()
}

fn packages_style() -> ProgressStyle {
    ProgressStyle::with_template(
        "{spinner:.green} {msg} [{bar:20.cyan/blue}] {bytes}/{total_bytes} ({percent}%)",
    )
    .unwrap()
    .progress_chars("█░")
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
    let multi = MultiProgress::with_draw_target(ProgressDrawTarget::stdout());

    let (ok, failures) = stream::iter(repositories.into_iter().map(|repository| {
        let client = client.clone();
        let multi = &multi;
        async move { process_repository(repository, client, config, multi).await }
    }))
    .buffer_unordered(10)
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

async fn process_repository(
    repository: Repository,
    client: Client,
    config: &Configuration,
    multi: &MultiProgress,
) -> Result<(), String> {
    let label = repo_label(&repository);
    say(multi, format!("==> {label}"));

    let pb = multi.add(
        ProgressBar::new_spinner()
            .with_style(spinner_style())
            .with_message("descargando InRelease"),
    );
    pb.enable_steady_tick(Duration::from_millis(80));

    let result = run_repository(repository, client, config, multi, &pb).await;
    pb.finish_and_clear();

    match result {
        Ok(()) => {
            say(multi, "  ✓ Packages verificado (SHA256)");
            Ok(())
        }
        Err(e) => {
            let mensaje = format!("{label}: {e}");
            fail(multi, format!("  ✗ {mensaje}"));
            Err(mensaje)
        }
    }
}

async fn run_repository(
    repository: Repository,
    client: Client,
    config: &Configuration,
    multi: &MultiProgress,
    pb: &ProgressBar,
) -> Result<(), String> {
    let downloaded = download_inrelease(repository, client.clone(), config, pb)
        .await
        .map_err(|e| e.to_string())?;

    pb.set_message("verificando firma");
    let procesado = task::spawn_blocking(move || process_inrelease(downloaded))
        .await
        .map_err(|e| format!("Error en spawn_blocking: {e}"))?;

    let (downloaded, package_info) = procesado.map_err(|e| e.to_string())?;
    say(multi, "  ✓ InRelease verificado");

    pb.set_length(package_info.size() as u64);
    pb.set_position(0);
    pb.set_message("Packages");
    pb.set_style(packages_style());

    download_packages_file(
        downloaded.repository,
        package_info,
        config,
        client,
        pb,
        multi,
    )
    .await
    .map_err(|e| e.to_string())
}

async fn download_inrelease(
    repository: Repository,
    client: Client,
    config: &Configuration,
    pb: &ProgressBar,
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
            pb,
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
    pb: &ProgressBar,
    multi: &MultiProgress,
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
                    say(
                        multi,
                        format!(
                            "El fichero Packages ya existe con el hash correcto, se omite la descarga: {file_path}"
                        ),
                    );
                    pb.set_position(package_info.size() as u64);
                    return Ok(());
                }
                Ok(Ok(_)) => {
                    say(
                        multi,
                        "El fichero Packages existe pero el hash no coincide, volviendo a descargar...",
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
            pb,
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
                fail(
                    multi,
                    format!("No se pudo eliminar el fichero corrupto {downloaded_filepath}: {e}"),
                );
            }
            continue;
        }

        return Ok(());
    }

    Err(mirror_error("Packages", &causas).into())
}
