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
) -> Result<String, Box<dyn std::error::Error>> {
    let response = client.get(url).send().await?.error_for_status()?;

    if let Some(content_length) = response.content_length() {
        pb.set_length(content_length);
    }
    pb.set_position(0);

    let mut stream = response.bytes_stream();

    let filepath = format!("{}/{}", download_path, filename);
    let mut file = File::create(&filepath).await?;

    let mut writer = BufWriter::new(&mut file);
    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        pb.set_position(pb.position() + bytes.len() as u64);
        writer.write_all(&bytes).await?;
    }
    writer.flush().await?;
    Ok(filepath)
}
