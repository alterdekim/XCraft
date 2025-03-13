use std::sync::{Arc, Mutex};
use rand::{distr::Alphanumeric, Rng};
use reqwest::{Client, Response};
use tokio::{fs::File, io::AsyncWriteExt};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::{mpsc};
use crate::launcher::Launcher;

pub fn random_string(len: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

pub fn download_file(url: &str, file_path: &str, size: u64, sender: UnboundedSender<(u8, String)>) -> Result<(), Box<dyn std::error::Error>> {
    let url = url.to_string();
    let file_path = file_path.to_string();
    tokio::spawn( async move {
        let client = Client::new();
        let mut response: Response = client.get(url).send().await.unwrap();

        if response.status().is_success() {
            let mut file = File::create(file_path).await.unwrap();
            let mut cur_size = 0;
            while let Some(chunk) = response.chunk().await.unwrap() {
                cur_size += chunk.len();
                sender.send((((cur_size / size as usize) * 100) as u8, "Downloading".to_string()) );
                let _ = file.write_all(&chunk).await;
            }
        } else {
            println!("Failed to download file: {}", response.status());
        }
    });
    Ok(())
}


