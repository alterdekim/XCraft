use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct LauncherCredentials {
    pub uuid: String,
    pub username: String,
    pub password: String
}

#[derive(Serialize, Deserialize)]
pub struct LauncherServer {
    pub domain: String, 
    pub port: u16,
    pub session_server_port: u16,
    pub credentials: LauncherCredentials
}

#[derive(Serialize, Deserialize)]
pub struct LauncherConfig {
    is_portable: bool,
    user_name: String,
    pub java_path: String,
    pub show_alpha: bool,
    pub show_beta: bool,
    pub show_snapshots: bool,
    pub ram_amount: u32,
    servers: Vec<LauncherServer>
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self { is_portable: Default::default(), user_name: Default::default(), java_path: "java".to_string(), show_alpha: true, show_beta: true, show_snapshots: false, ram_amount: 1024, servers: Default::default() }
    }
}

impl LauncherConfig {
    pub fn set_portable(&mut self, is_portable: bool) {
        self.is_portable = is_portable;
    }

    pub fn launcher_dir(&self) -> PathBuf {
        match self.is_portable {
            true => get_relative_launcher_dir(),
            false => get_absolute_launcher_dir()
        }
    }

    pub fn config_path(&self) -> PathBuf {
        let mut p = self.launcher_dir();
        p.push("config.toml");
        p
    }

    pub fn user_name(&self) -> &str {
        &self.user_name
    }

    pub fn set_username(&mut self, user_name: String) {
        self.user_name = user_name;
    }

    pub fn add_server(&mut self, server: LauncherServer) {
        self.servers.push(server);
    }
}

pub fn get_relative_launcher_dir() -> PathBuf {
    let mut p = std::env::current_dir().unwrap();
    p.push("xcraft");
    p
}

fn get_absolute_launcher_dir() -> PathBuf {
    let mut p = dirs::data_dir().unwrap();
    p.push("xcraft");
    p
}