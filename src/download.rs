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
    let http = |source: reqwest::Error| NyaError::Http { source };

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

    let write_result = async {
        let file = File::create(&temp_path)
            .await
            .map_err(|source| NyaError::io("guardar", &temp_path, source))?;
        let mut writer = BufWriter::new(file);
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(http)?;
            pb.set_position(pb.position() + bytes.len() as u64);
            writer
                .write_all(&bytes)
                .await
                .map_err(|source| NyaError::io("guardar", &temp_path, source))?;
        }
        writer
            .flush()
            .await
            .map_err(|source| NyaError::io("guardar", &temp_path, source))?;
        Ok::<(), NyaError>(())
    }
    .await;

    match write_result {
        Ok(()) => {
            tokio::fs::rename(&temp_path, &filepath)
                .await
                .map_err(|source| NyaError::io("guardar", &filepath, source))?;
            Ok(filepath)
        }
        Err(e) => {
            let _ = tokio::fs::remove_file(&temp_path).await;
            Err(e)
        }
    }
}
