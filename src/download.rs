use crate::error::NyaError;
use futures::stream::StreamExt;
use indicatif::ProgressBar;
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_path_for(filepath: &Path) -> PathBuf {
    let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut temp = filepath.as_os_str().to_owned();
    temp.push(format!(".{}.{}.part", std::process::id(), n));
    PathBuf::from(temp)
}

pub async fn download_file(
    url: &str,
    filename: &str,
    download_path: &Path,
    client: Client,
    pb: &ProgressBar,
) -> Result<PathBuf, NyaError> {
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

    let filepath = download_path.join(filename);
    let temp_path = temp_path_for(&filepath);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_path_for_is_unique_per_call() {
        let first = temp_path_for(Path::new("/etc/nya/host_dists_noble_InRelease"));
        let second = temp_path_for(Path::new("/etc/nya/host_dists_noble_InRelease"));

        assert_ne!(first, second, "cada descarga necesita su propio .part");
        let first = first.to_string_lossy();
        assert!(
            first.contains(&std::process::id().to_string()),
            "el temporal debe llevar el pid: {first}"
        );
        assert!(first.ends_with(".part"), "sufijo .part: {first}");
    }
}
