use std::sync::{Arc, Mutex};
use futures::AsyncReadExt;
use rand::{distr::Alphanumeric, Rng};
use tokio::{fs::File, io::AsyncWriteExt};
use tokio::sync::mpsc::{Sender, UnboundedReceiver, UnboundedSender};
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
        let mut res = surf::get(url).await.unwrap();
    
        let total_size = res.len().unwrap_or(0); // Total size in bytes (if available)
        let mut downloaded = 0;
        let mut buf = vec![0; 8192]; // Buffer for reading chunks
        
        let mut file = File::create(file_path).await.unwrap();

        let mut r= res.take_body().into_reader();
        while let Ok(n) = r.read(&mut buf).await {
            if n == 0 {
                break;
            }
            downloaded += n;
            
            file.write(&buf[..n]).await;
        }
        sender.send((downloaded, status.clone()));
    });
    Ok(())
}


