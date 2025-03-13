use rand::{distr::Alphanumeric, Rng};
use reqwest::{Client, Response};
use tokio::{fs::File, io::AsyncWriteExt};

pub fn random_string(len: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

pub async fn download_file(url: &str, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let mut response: Response = client.get(url).send().await?;
    
    if response.status().is_success() {
        let mut file = File::create(file_path).await?;
        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk).await?;
        }
    } else {
        println!("Failed to download file: {}", response.status());
    }

    Ok(())
}