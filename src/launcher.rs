use core::str;

use crate::config::LauncherConfig;


#[derive(Default)]
pub struct Launcher {
    pub config: LauncherConfig
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