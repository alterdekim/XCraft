use core::str;
use std::io::Cursor;
use base64::{encode, Engine};
use base64::prelude::BASE64_STANDARD;
use tokio::fs::File;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use crate::minecraft::versions::Version;
use crate::{config::LauncherConfig, minecraft::versions::VersionConfig, util};

const JAVA_ARGS: [&str; 23] = ["-Xms1024M", 
"-XX:+UnlockExperimentalVMOptions", 
"-XX:+DisableExplicitGC",
"-XX:MaxGCPauseMillis=200",
"-XX:+AlwaysPreTouch",
"-XX:+ParallelRefProcEnabled",
"-XX:+UseG1GC",
"-XX:G1NewSizePercent=30",
"-XX:G1MaxNewSizePercent=40",
"-XX:G1HeapRegionSize=8M",
"-XX:G1ReservePercent=20",
"-XX:InitiatingHeapOccupancyPercent=15",
"-XX:G1HeapWastePercent=5",
"-XX:G1MixedGCCountTarget=4",
"-XX:G1MixedGCLiveThresholdPercent=90",
"-XX:G1RSetUpdatingPauseTimePercent=5",
"-XX:+UseStringDeduplication", "-Xmx1024M", "-Dfile.encoding=UTF-8", "-Dfml.ignoreInvalidMinecraftCertificates=true", "-Dfml.ignorePatchDiscrepancies=true", "-Djava.net.useSystemProxies=true", "-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump"];

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
        let _ = std::fs::write(self.config.config_path(), toml::to_string_pretty(&self.config).unwrap());
    }

    pub fn init_config(&mut self, user_name: String) {
        self.load_config();
        self.config.user_name = user_name;
        self.config.user_secret = crate::util::random_string(32);
        self.save_config();
    }

    pub fn get_instances_list(&self) -> Vec<(String, String, String)> {
        let mut v = Vec::new();
        let mut instances = self.config.launcher_dir();
        instances.push("instances");
        if let Ok(entries) = std::fs::read_dir(instances) {
            for entry in entries {
                if entry.is_err() { continue; }
                let entry = entry.unwrap();
                if !entry.metadata().unwrap().is_dir() { continue; }
                let mut p = entry.path();
                p.push("client.json");
                if let Ok(data) = std::fs::read(p) {
                    let config: VersionConfig = serde_json::from_slice(&data).unwrap();
                    v.push((config.id, config.r#type,  format!("data:image/png;base64,{}", BASE64_STANDARD.encode(include_bytes!("www/icons/alpha.png")))));
                }
            }
        }
        v
    }

    pub async fn launch_instance(&self, instance_name: String) {
        let mut instances = self.config.launcher_dir();
        instances.push("instances");
        instances.push(&instance_name);
        instances.push("client.json");

        let mut client_jar = self.config.launcher_dir();
        client_jar.push("instances");
        client_jar.push(&instance_name);
        client_jar.push("client.jar");

        let mut cmd = Vec::new();
        //cmd.push("java".to_string());
        
        for arg in JAVA_ARGS {
            cmd.push(arg.to_string());
        }

        let mut natives_path = self.config.launcher_dir();
        natives_path.push("instances");
        natives_path.push(&instance_name);
        natives_path.push("natives");

        cmd.push(["-Djava.library.path=", natives_path.to_str().unwrap() ].concat());
        cmd.push(["-Dminecraft.client.jar=", client_jar.to_str().unwrap()].concat());
        cmd.push("-cp".to_string());

        if let Ok(data) = std::fs::read(&instances) {
            let config: VersionConfig = serde_json::from_slice(&data).unwrap();
            let mut libraries_cmd = Vec::new();
            for library in config.libraries {
                if let Some(classifier) = &library.downloads.classifiers {
                    if let Some(natives) = &classifier.natives {
                        let rel_path = &natives.path;
                        let mut libs = self.config.launcher_dir();
                        libs.push("libraries");
                        let rel_path = [libs.to_str().unwrap(), "\\", &rel_path.replace("/", "\\")].concat();
                        let data = std::fs::read(rel_path).unwrap();

                        zip_extract::extract(Cursor::new(data), &natives_path, true);
                    }
                } else {
                    let mut libs = self.config.launcher_dir();
                    libs.push("libraries");
                    libs.push(library.to_pathbuf_file());
                    libraries_cmd.push([libs.to_str().unwrap(), ";"].concat());
                }
            }
            libraries_cmd.push(client_jar.to_str().unwrap().to_string());
            cmd.push(libraries_cmd.concat());
            cmd.push(config.mainClass.clone());
        }

        let mut game_dir = self.config.launcher_dir();
        game_dir.push("instances");
        game_dir.push(&instance_name);
        game_dir.push("data");

        cmd.push(["--username", "getter", "--version", "1.9.4", "--gameDir", game_dir.to_str().unwrap(), "--assetsDir", "D:\\Documents\\RustroverProjects\\XCraft\\xcraft\\assets", "--assetIndex", "1.9", "--uuid", "51820246d9fe372b81592602a5239ad9", "--accessToken", "51820246d9fe372b81592602a5239ad9", "--userProperties", "{}", "--userType", "legacy", "--width", "925", "--height", "530"].join(" "));
    }


    pub async fn new_vanilla_instance(&mut self, config: VersionConfig, version_object: &Version, sender: UnboundedSender<(u8, String)>) {
        
        let (sx, mut rx) = mpsc::unbounded_channel();
        
        let root = self.config.launcher_dir();
        let mut instances = root.clone();
        instances.push("instances");
        instances.push(&config.id);

        let _ = std::fs::create_dir_all(&instances);

        instances.push("client.jar");

        let mut overall_size = config.downloads.client.size as usize;
        let mut cnt = 0;

        let client_jar_url = config.downloads.client.url;

        let mut client_json_path = root.clone();
        client_json_path.push("instances");
        client_json_path.push(config.id);
        client_json_path.push("client.json");

        let _ = util::download_file(&version_object.url, client_json_path.to_str().unwrap(), sx.clone(), "Downloading client.json");
        cnt += 1;

        let _ = util::download_file(&client_jar_url, instances.to_str().unwrap(), sx.clone(), "Downloading client.jar");
        cnt += 1;

        let mut libraries = root.clone();
        libraries.push("libraries");

        for i in 0..config.libraries.len() {
            let library = &config.libraries[i];
            if let Some(artifact) = &library.downloads.artifact {
                overall_size += artifact.size as usize;
                let mut dl_path = libraries.clone();
                let mut dl_pp = libraries.clone();
                dl_pp.push(library.to_pathbuf_path());
                let _ = std::fs::create_dir_all(dl_pp);
                dl_path.push(library.to_pathbuf_file());
                let _ = util::download_file(&artifact.url, dl_path.to_str().unwrap(), sx.clone(), "Downloading libraries");
                cnt += 1;
            }

            if let Some(classifiers) = &library.downloads.classifiers {
                if let Some(natives) = &classifiers.natives {
                    overall_size += natives.size as usize;
                    let mut dl_path = libraries.clone();
                    dl_path.push(&natives.path);
                    let t_p = dl_path.to_str().unwrap().split("/").collect::<Vec<&str>>();
                    let t_p = t_p[..t_p.len()-1].join("/");
                    let _ = std::fs::create_dir_all(&t_p);
                    let _ = util::download_file(&natives.url, dl_path.to_str().unwrap(), sx.clone(), "Downloading natives");
                    cnt += 1;
                }
            }
        }

        let mut assets_path = root.clone();
        assets_path.push("assets");

        let mut indexes = assets_path.clone();
        indexes.push("indexes");
        let _ = std::fs::create_dir_all(indexes);

        let mut objects = assets_path.clone();
        objects.push("objects");
        let _ = std::fs::create_dir_all(objects);

        let mut index = assets_path.clone();
        index.push(config.assetIndex.to_path());

        let _ = util::download_file(&config.assetIndex.url, index.to_str().unwrap(), sx.clone(), "Downloading assets indexes");
        cnt += 1;

        let asset_index = config.assetIndex.url;

        overall_size += config.assetIndex.size as usize;
        overall_size += config.assetIndex.totalSize as usize;

        let assets = crate::minecraft::assets::fetch_assets_list(&asset_index).await.unwrap().objects;

        for (_key, asset) in assets {
            let mut single_object = assets_path.clone();
            single_object.push(asset.to_path());

            let mut single_object_path = assets_path.clone();
            single_object_path.push(asset.to_small_path());
            std::fs::create_dir_all(single_object_path);

            util::download_file(&asset.to_url(), single_object.to_str().unwrap(), sx.clone(), "Downloading assets objects");
            cnt += 1;
        }

        tokio::spawn(async move {
            let mut current_size = 0;
            let mut current_cnt = 0;
            while let Some((size, status)) = rx.recv().await {
                current_size += size;
                current_cnt += 1;
                sender.send((((current_size as f32 / overall_size as f32) * 100.0) as u8, status));
                if current_cnt >= cnt {
                    sender.send((100, "_".to_string()));
                }
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