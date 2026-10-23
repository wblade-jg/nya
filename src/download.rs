use crate::error::NyaError;
use futures::stream::StreamExt;
use indicatif::ProgressBar;
use reqwest::Client;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};

pub async fn download_file(
    url: &str,
    filename: &str,
    download_path: &str,
    client: Client,
    pb: &ProgressBar,
) -> Result<String, NyaError> {
    let http = |source: reqwest::Error| NyaError::Http {
        url: url.to_string(),
        source,
    };

    let response = client
        .get(url)
        .send()
        .await
        .map_err(http)?
        .error_for_status()
        .map_err(http)?;

    if let Some(content_length) = response.content_length() {
        pb.set_length(content_length);
    }
    pb.set_position(0);

    let mut stream = response.bytes_stream();

    let filepath = format!("{}/{}", download_path, filename);
    let temp_path = format!("{filepath}.part");

    let escritura = async {
        let file = File::create(&temp_path)
            .await
            .map_err(|source| NyaError::StoreFile {
                path: temp_path.clone(),
                source,
            })?;
        let mut writer = BufWriter::new(file);
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(http)?;
            pb.set_position(pb.position() + bytes.len() as u64);
            writer
                .write_all(&bytes)
                .await
                .map_err(|source| NyaError::StoreFile {
                    path: temp_path.clone(),
                    source,
                })?;
        }
        writer.flush().await.map_err(|source| NyaError::StoreFile {
            path: temp_path.clone(),
            source,
        })?;
        Ok::<(), NyaError>(())
    }
    .await;

    match escritura {
        Ok(()) => {
            tokio::fs::rename(&temp_path, &filepath)
                .await
                .map_err(|source| NyaError::StoreFile {
                    path: filepath.clone(),
                    source,
                })?;
            Ok(filepath)
        }
        Err(e) => {
            let _ = tokio::fs::remove_file(&temp_path).await;
            Err(e)
        }
    }
}
