use core::str;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::sync::mpsc::{Sender, UnboundedReceiver, UnboundedSender};
use crate::{config::LauncherConfig, minecraft::versions::VersionConfig, util};

#[derive(Default)]
pub struct Launcher {
    pub config: LauncherConfig,
}

impl Launcher {

    pub fn is_portable(&self) -> bool {
        crate::config::get_relative_launcher_dir().exists()
    }

    pub fn is_config_exist(&self) -> bool {
        self.config.config_path().exists()
    }

    pub fn load_config(&mut self) {
        if self.is_config_exist() {
            self.config = toml::from_str(
                str::from_utf8(&std::fs::read(self.config.config_path()).unwrap()).unwrap()).unwrap();
        }
    }

    pub fn save_config(&self) {
        std::fs::write(self.config.config_path(), toml::to_string_pretty(&self.config).unwrap());
    }

    pub fn init_config(&mut self, user_name: String) {
        self.load_config();
        self.config.user_name = user_name;
        self.config.user_secret = crate::util::random_string(32);
        self.save_config();
    }

    pub async fn new_vanilla_instance(&mut self, config: VersionConfig, sender: UnboundedSender<(u8, String)>) {
        
        let (sx, mut rx) = mpsc::unbounded_channel();
        
        let root = self.config.launcher_dir();
        let mut instances = root.clone();
        instances.push("instances");
        instances.push(config.id);

        std::fs::create_dir_all(&instances);

        instances.push("client.jar");

        let mut overall_size = config.downloads.client.size as usize;

        let client_jar_url = config.downloads.client.url;

        util::download_file(&client_jar_url, instances.to_str().unwrap(), config.downloads.client.size, sx.clone(), "Downloading client.jar");

        let mut libraries = root.clone();
        libraries.push("libraries");

        for i in 0..config.libraries.len() {
            let library = &config.libraries[i];
            if let Some(artifact) = &library.downloads.artifact {
                overall_size += artifact.size as usize;
                let mut dl_path = libraries.clone();
                let mut dl_pp = libraries.clone();
                dl_pp.push(library.to_pathbuf_path());
                std::fs::create_dir_all(dl_pp);
                dl_path.push(library.to_pathbuf_file());
                util::download_file(&artifact.url, dl_path.to_str().unwrap(), config.downloads.client.size, sx.clone(), "Downloading libraries");
            }
        }

        tokio::spawn(async move {
            let mut current_size = 0;
            while let Some((size, status)) = rx.recv().await {
                current_size += size;
                sender.send((((current_size as f32 / overall_size as f32) * 100.0) as u8, status));
            }
        });
    }

    pub fn init_dirs(&self) {
        let root = self.config.launcher_dir();
        std::fs::create_dir_all(&root);
        // instances assets libraries config.toml servers credentials
        let mut instances = root.clone();
        instances.push("instances");

        let mut assets = root.clone();
        assets.push("assets");

        let mut libraries = root.clone();
        libraries.push("libraries");

        let mut servers = root.clone();
        servers.push("servers");

        let mut credentials = root.clone();
        credentials.push("credentials");

        std::fs::create_dir_all(&instances);
        std::fs::create_dir_all(&assets);
        std::fs::create_dir_all(&libraries);
        std::fs::create_dir_all(&servers);
        std::fs::create_dir_all(&credentials);
    }
}