use futures::AsyncReadExt;
use rand::{distr::Alphanumeric, Rng};
use tokio::{fs::File, io::AsyncWriteExt};
use tokio::sync::mpsc::UnboundedSender;

pub fn random_string(len: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

pub fn download_file(url: &str, file_path: &str, sender: UnboundedSender<(usize, String)>, status: &str) -> Result<(), Box<dyn std::error::Error>> {
    let url = url.to_string();
    let file_path = file_path.to_string();
    let status = status.to_string();
    tokio::spawn( async move {
        if let Ok(mut res) = surf::get(url).await {
            let mut downloaded = 0;
            let mut buf = vec![0; 8192]; // Buffer for reading chunks
            
            let mut file = File::create(file_path).await.unwrap();

            let mut r= res.take_body().into_reader();
            while let Ok(n) = r.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                downloaded += n;
                
                let _ = file.write(&buf[..n]).await;
            }
            let _ = sender.send((downloaded, status.clone()));
        } else {
            let _ = sender.send((0, status.clone()));
        }
    });
    Ok(())
}


