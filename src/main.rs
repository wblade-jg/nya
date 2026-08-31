use futures::stream::{self, StreamExt};
use nya::{
    download,
    parser::{self, Repository},
    signature,
};
use reqwest::Client;
use std::sync::LazyLock;
use tokio::task;

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
    
    let filename = parser::format_url("_", &download_url).unwrap();
    let inrelease_path = download::download_file(&download_url, &filename, client)
        .await
        .map_err(|e| {
            eprintln!("Error en la descarga: {}", e);
            e
        })?;

    let signature_path = repository.signed_by();
    let target_package_path = repository.packages_path(&*ARCHITECTURE);

    println!("Descargado: {}", inrelease_path);
    Ok(DownloadedInRelease {
        inrelease_path,
        signature_path,
        target_package_path,
    })
}

#[derive(Debug, Clone)]
struct DownloadedInRelease {
    inrelease_path: String,
    signature_path: String,
    target_package_path: String,
}

fn process_inrelease(
    downloaded: DownloadedInRelease,
) -> Result<PackageFileInfo, Box<dyn std::error::Error + Send + Sync>> {
    let content =
        signature::validate_signature_file(&downloaded.inrelease_path, &downloaded.signature_path)?;

    get_package_file_info(&content, &downloaded.target_package_path)
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

struct PackageFileInfo {
    hash: String,
    size: usize,
    path: String,
}

impl PackageFileInfo {
    fn new(hash: String, size: usize, path: String) -> Self {
        PackageFileInfo { hash, size, path }
    }
}

fn get_package_file_info(
    content: &str,
    target_path: &str,
) -> Result<PackageFileInfo, Box<dyn std::error::Error + Send + Sync>> { 
    let hash_type = HashType::SHA256;
    
    let block_hash_start = content
        .find(hash_type.as_str())
        .ok_or_else(|| format!("No se encontro la seccion {} en el InRelease", hash_type.as_str()))?; 

    let content_from_hash_start = &content[block_hash_start..];

    for line in content_from_hash_start.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parts = trimmed.split_whitespace().collect::<Vec<_>>();
        if parts.len() != 3 {
            continue;
        }

        let hash = parts[0].to_string();
        let size = parts[1].parse::<usize>()?;
        let path = parts[2].to_string();

        if path == target_path || path.ends_with(target_path) { 
            return Ok(PackageFileInfo::new(hash, size, path));
        }
    }

    Err(format!(
        "No se encontró {} en la sección {}",
        target_path,
        hash_type.as_str()
    )
    .into())
}

enum HashType {
    MD5,
    SHA1,
    SHA256,
    SHA512,
}

impl HashType {
    fn as_str(&self) -> &'static str {
        match self {
            HashType::MD5 => "MD5",
            HashType::SHA1 => "SHA1",
            HashType::SHA256 => "SHA256",
            HashType::SHA512 => "SHA512",
        }
    }

    fn size(&self) -> usize {
        match self {
            HashType::MD5 => 32,
            HashType::SHA1 => 40,
            HashType::SHA256 => 64,
            HashType::SHA512 => 128,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn verify_package_file_info() {
        let hash_type = HashType::SHA256;
        let inrelease_content = r#"
          Origin: Test
          Suite: stable
          
          SHA256:
           a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2 12345 main/binary-amd64/Packages
           f1e2d3c4b5a697887766554433221100ffeeddccbbaa99887766554433221100 67890 main/binary-amd64/Packages.gz
          
          MD5Sum:
           aaaa 12345 main/binary-amd64/Packages
          "#;
        let target_path = "main/binary-amd64/Packages";
        let result = get_package_file_info(inrelease_content, target_path);
        assert!(result.is_ok(), "get_package_file_info should return Ok");
        let package_file_info = result.unwrap();
        assert_eq!(
            package_file_info.hash.len(),
            hash_type.size(),
            "hash should be {} characters long",
            hash_type.size()
        );
    }
}
