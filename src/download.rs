use futures::stream::StreamExt;
use reqwest::Client;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};

pub async fn download_file(
    url: &str,
    filename: &str,
    download_path: &str, 
    client: Client,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut stream = client.get(url).send().await?.bytes_stream();

    let filepath = format!("{}/{}", download_path, filename);
    let mut file = File::create(&filepath).await?;

    let mut writer = BufWriter::new(&mut file);
    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        writer.write_all(&bytes).await?;
    }
    writer.flush().await?;
    Ok(filepath)
}
