use crate::configuration::{
    Configuration, architecture, resolve_cache_dir, resolve_repositories_file,
};
use crate::error::NyaError;
use crate::types::{PackageFileInfo, Repository};
use crate::{download, integrity, package_info, parser, url};
use futures::stream::{self, StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::task;

fn repo_label(repository: &Repository) -> String {
    let first_uri = repository
        .urls()
        .next()
        .map(String::as_str)
        .unwrap_or("repositorio sin URIs");
    format!("{first_uri} ({})", repository.suite())
}

fn say(multi: &MultiProgress, line: impl AsRef<str>) {
    if multi.is_hidden() {
        println!("{}", line.as_ref());
    } else {
        let _ = multi.println(line.as_ref());
    }
}

fn fail(multi: &MultiProgress, line: impl AsRef<str>) {
    let line = line.as_ref().to_string();
    multi.suspend(move || eprintln!("{line}"));
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

async fn hash_file(path: &Path) -> Result<String, NyaError> {
    let path = path.to_path_buf();
    task::spawn_blocking(move || integrity::calculate_file_hash(path))
        .await
        .map_err(|e| NyaError::Join(e.to_string()))?
}

async fn execute_pipeline(
    repositories: Vec<Repository>,
    config: &Configuration,
) -> Result<(), NyaError> {
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

    resolve_cache_dir(&config.download_path)?;

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(NyaError::HttpClient)?;
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
        return Err(NyaError::UpdateFailed {
            failed: failures,
            total,
        });
    }

    println!("==> Hecho: {ok}/{total} correcto");
    Ok(())
}

pub async fn update(config: &Configuration) -> Result<(), NyaError> {
    let repositories_filepath = resolve_repositories_file(&config.repositories_filepath)?;
    let repositories = parser::read_repositories_from_file(&repositories_filepath).await?;
    execute_pipeline(repositories, config).await
}

async fn process_repository(
    repository: Repository,
    client: Client,
    config: &Configuration,
    multi: &MultiProgress,
) -> Result<(), NyaError> {
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
            fail(multi, format!("  ✗ {label}: {e}"));
            Err(e)
        }
    }
}

async fn run_repository(
    repository: Repository,
    client: Client,
    config: &Configuration,
    multi: &MultiProgress,
    pb: &ProgressBar,
) -> Result<(), NyaError> {
    let inrelease_path = download_inrelease(&repository, client.clone(), config, pb).await?;

    pb.set_message("verificando firma");
    let key_path = repository.signed_by().to_string();
    let target_path = repository.target_package_path(architecture()?);
    let processed =
        task::spawn_blocking(move || process_inrelease(&inrelease_path, &key_path, &target_path))
            .await
            .map_err(|e| NyaError::Join(e.to_string()))?;

    let package_info = processed?;
    say(multi, "  ✓ InRelease verificado");

    pb.set_length(package_info.size() as u64);
    pb.set_position(0);
    pb.set_message("Packages");
    pb.set_style(packages_style());

    download_packages_file(&repository, package_info, config, client, pb, multi).await
}

async fn download_inrelease(
    repository: &Repository,
    client: Client,
    config: &Configuration,
    pb: &ProgressBar,
) -> Result<PathBuf, NyaError> {
    let suite = repository.suite();
    let mut causes: Vec<String> = Vec::new();

    for url in repository.urls() {
        let download_url = format!("{}/dists/{}/InRelease", url, suite);
        let Some(filename) = url::format_url("_", &download_url) else {
            causes.push(format!("{download_url}: URL no válida"));
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
            Ok(downloaded_filepath) => return Ok(downloaded_filepath),
            Err(e) => causes.push(format!("{download_url}: {e}")),
        }
    }

    Err(NyaError::mirrors_failed("InRelease", &causes))
}

fn process_inrelease(
    inrelease_path: &Path,
    key_path: &str,
    target_path: &str,
) -> Result<PackageFileInfo, NyaError> {
    let content = integrity::validate_signature_file(inrelease_path, key_path)?;
    package_info::check_valid_until(&content, inrelease_path)?;
    package_info::get_package_file_info(&content, target_path)
}

async fn download_packages_file(
    repository: &Repository,
    package_info: PackageFileInfo,
    config: &Configuration,
    client: Client,
    pb: &ProgressBar,
    multi: &MultiProgress,
) -> Result<(), NyaError> {
    let suite = repository.suite();
    let mut causes: Vec<String> = Vec::new();

    for base_url in repository.urls() {
        let download_url = format!("{}/dists/{}/{}", base_url, suite, package_info.path());
        let Some(filename) = url::format_url("_", &download_url) else {
            causes.push(format!("{download_url}: URL no válida"));
            continue;
        };
        let file_path = config.download_path.join(&filename);

        if file_path.exists() {
            match hash_file(&file_path).await {
                Ok(existing_hash) if existing_hash == package_info.hash() => {
                    say(
                        multi,
                        format!(
                            "El fichero Packages ya existe con el hash correcto, se omite la descarga: {}",
                            file_path.display()
                        ),
                    );
                    pb.set_position(package_info.size() as u64);
                    return Ok(());
                }
                Ok(_) => say(
                    multi,
                    "El fichero Packages existe pero el hash no coincide, volviendo a descargar...",
                ),
                Err(e) => causes.push(format!("{download_url}: no se pudo calcular el hash: {e}")),
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
                causes.push(format!("{download_url}: {e}"));
                continue;
            }
        };

        let downloaded_hash = match hash_file(&downloaded_filepath).await {
            Ok(hash) => hash,
            Err(e) => {
                causes.push(format!("{download_url}: no se pudo calcular el hash: {e}"));
                continue;
            }
        };

        if downloaded_hash != package_info.hash() {
            causes.push(format!(
                "{download_url}: el hash no coincide (esperado {}, obtenido {downloaded_hash})",
                package_info.hash()
            ));
            if let Err(e) = std::fs::remove_file(&downloaded_filepath) {
                fail(
                    multi,
                    format!(
                        "No se pudo eliminar el fichero corrupto {}: {e}",
                        downloaded_filepath.display()
                    ),
                );
            }
            continue;
        }

        return Ok(());
    }

    Err(NyaError::mirrors_failed("Packages", &causes))
}
