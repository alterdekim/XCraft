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

pub fn download_file(url: &str, file_path: &str, size: u64, sender: UnboundedSender<(usize, String)>, status: &str) -> Result<(), Box<dyn std::error::Error>> {
    let url = url.to_string();
    let file_path = file_path.to_string();
    let status = status.to_string();
    tokio::spawn( async move {
        let client = Client::new();
        let mut response: Response = client.get(url).send().await.unwrap();

        if response.status().is_success() {
            let mut file = File::create(file_path).await.unwrap();
            while let Some(chunk) = response.chunk().await.unwrap() {
                if let Err(e) = sender.send((chunk.len(), status.clone()) ) {
                    println!("SendError: {}", e);
                }
                let _ = file.write(&chunk).await;
            }
        } else {
            println!("Failed to download file: {}", response.status());
        }
    });
    Ok(())
}


