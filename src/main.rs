use futures::stream::{self, StreamExt};
use nya::{
    download,
    parser::{self, Repository},
    package_info, signature, url,
};
use reqwest::Client;
use std::sync::LazyLock;
use tokio::task;
use nya::types::{DownloadedInRelease, PackageFileInfo};

const REPOSITORIES_FILEPATH: &str = "sources";
const ARCHITECTURE: LazyLock<String> = LazyLock::new(|| match std::env::consts::ARCH {
    "x86_64" => String::from("amd64"),
    "aarch64" => String::from("arm64"),
    _ => panic!("Arquitectura no soportada"),
});

async fn download_inrelease(
    repository: Repository,
    client: Client,
) -> Result<DownloadedInRelease, Box<dyn std::error::Error>> {
    let download_url = repository.inrelease_path();

    let filename = url::format_url("_", &download_url).unwrap();
    let inrelease_path = download::download_file(&download_url, &filename, client)
        .await
        .map_err(|e| {
            eprintln!("Error en la descarga: {}", e);
            e
        })?;

    let signature_path = repository.signed_by();
    let target_package_path = repository.packages_path(&*ARCHITECTURE);

    println!("Descargado: {}", inrelease_path);
    Ok(DownloadedInRelease::new(
        inrelease_path,
        signature_path,
        target_package_path,
    ))
}

fn process_inrelease(
    downloaded: DownloadedInRelease,
) -> Result<PackageFileInfo, Box<dyn std::error::Error + Send + Sync>> {
    let content =
        signature::validate_signature_file(downloaded.inrelease_path(), downloaded.signature_path())?;

    package_info::get_package_file_info(&content, downloaded.target_package_path())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    match parser::read_repositories_from_file(REPOSITORIES_FILEPATH).await {
        Ok(repositories) => {
            let client = Client::new();

            let pipeline = stream::iter(
                repositories
                    .into_iter()
                    .map(|repository| download_inrelease(repository, client.clone())),
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
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }

    Ok(())
}
