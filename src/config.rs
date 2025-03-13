use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct LauncherConfig {
    is_portable: bool,
    pub user_name: String,
    pub user_secret: String
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