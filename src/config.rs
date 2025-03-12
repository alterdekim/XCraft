use std::path::PathBuf;

#[derive(Default)]
pub struct LauncherConfig {
    is_portable: bool
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
}

fn get_relative_launcher_dir() -> PathBuf {
    std::env::current_dir().unwrap()
}

fn get_absolute_launcher_dir() -> PathBuf {
    let mut p = dirs::data_dir().unwrap();
    p.push("xcraft");
    p
}