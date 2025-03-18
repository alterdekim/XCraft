use core::str;
use std::io::Cursor;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use crate::config::{LauncherCredentials, LauncherServer};
use crate::minecraft;
use crate::minecraft::session::SignUpResponse;
use crate::minecraft::versions::Version;
use crate::{config::LauncherConfig, minecraft::versions::VersionConfig, util};

const JAVA_ARGS: [&str; 22] = ["-Xms512M", 
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
"-XX:+UseStringDeduplication", "-Dfile.encoding=UTF-8", "-Dfml.ignoreInvalidMinecraftCertificates=true", "-Dfml.ignorePatchDiscrepancies=true", "-Djava.net.useSystemProxies=true", "-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump"];

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
        self.config.set_username(user_name);
        self.save_config();
    }

    fn save_server_info(&mut self, uuid: String, username: String, password: String, domain: String, session_server_port: u16, server_port: u16) -> (bool, &str) {
        self.config.add_server(LauncherServer {
            domain,
            port: server_port,
            session_server_port,
            credentials: LauncherCredentials {
                uuid,
                username,
                password
            }
        });
        self.save_config();
        (true, "You are successfully registered")
    }

    pub async fn register_user_server(&mut self, server: String, username: String, password: String) -> (bool, &str) {
        let mut session_server_port: u16 = 8999;
        let mut server_port: u16 = 25565;
        let mut domain = server.clone();
        if let Some(index) = server.find("#") {
            let (a,b) = server.split_at(index+1);
            session_server_port = b.parse().unwrap();
            domain = a[..a.len()-1].to_string();
        }

        if let Some(index) = domain.find(":") {
            let dmc = domain.clone();
            let (a,b) = dmc.split_at(index+1);
            domain = a[..a.len()-1].to_string();
            server_port = b.parse().unwrap();
        }

        println!("Server information: {}:{} session={}", domain, server_port, session_server_port);

        match minecraft::session::try_signup(domain.clone(), session_server_port, username.clone(), password.clone(), self.config.allow_http).await {
            Ok(status) => match status {
                SignUpResponse::ServerError => (false, "Internal server error"),
                SignUpResponse::BadCredentials => (false, "Username or password is not valid"),
                SignUpResponse::UserAlreadyExists => (false, "User already exists"),
                SignUpResponse::Registered(uuid) => self.save_server_info(uuid, username, password, domain, session_server_port, server_port)
            }
            Err(_e) => (false, "Internal server error")
        }
    }

    pub async fn get_servers_list(&self) -> Vec<(String, String, Option<String>)> {
        let mut v = Vec::new();
        let servers = self.config.servers();
        for server in servers {
            v.push((server.domain.clone(), server.credentials.username.clone(), minecraft::server::get_server_icon(&server.domain, server.port).await.unwrap_or(None)));
        }
        v
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
                    let c_type = config.r#type;
                    let c_type = c_type.as_str();
                    v.push((config.id, c_type.to_string(),  format!("data:image/png;base64,{}", BASE64_STANDARD.encode(match c_type {
                        "old_alpha" => include_bytes!("www/icons/alpha.png").to_vec(),
                        "old_beta" => include_bytes!("www/icons/beta.png").to_vec(),
                        "release" | "snapshot" => include_bytes!("www/icons/release.png").to_vec(),
                        _ => include_bytes!("www/icons/glowstone.png").to_vec()
                    }))));
                }
            }
        }
        v
    }

    pub fn get_screenshots(&self) -> Vec<(String, String)> {
        let mut v = Vec::new();
        let mut instances = self.config.launcher_dir();
        instances.push("instances");
        if let Ok(entries) = std::fs::read_dir(instances) {
            for entry in entries {
                if entry.is_err() { continue; }
                let entry = entry.unwrap();
                if !entry.metadata().unwrap().is_dir() { continue; }
                let mut p = entry.path();
                p.push("data");
                p.push("screenshots");
                if !p.exists() { continue; }
                if let Ok(screenshots) = std::fs::read_dir(p) {
                    let tmp = screenshots;
                    for screenshot in tmp.flatten() {
                        if screenshot.file_name().to_str().unwrap().ends_with("png") {
                            v.push((screenshot.path().to_str().unwrap().to_string(), format!("data:image/png;base64,{}", BASE64_STANDARD.encode(std::fs::read(screenshot.path()).unwrap()))));
                        }
                    }
                }
            }
        }
        v
    }

    pub async fn launch_instance(&self, instance_name: String, sender: UnboundedSender<String>, special_server: Option<&LauncherServer> ) {

        let mut username = self.config.user_name();
        let mut uuid = util::random_string(32);
        let mut token = util::random_string(32);

        if let Some(server) = special_server {
            username = &server.credentials.username;
            uuid = server.credentials.uuid.clone();
            token = server.credentials.password.clone();
        }
        
        let mut instances = self.config.launcher_dir();
        instances.push("instances");
        instances.push(&instance_name);
        instances.push("client.json");

        let mut client_jar = self.config.launcher_dir();
        client_jar.push("instances");
        client_jar.push(&instance_name);
        client_jar.push("client.jar");

        let mut instance_dir = self.config.launcher_dir();
        instance_dir.push("instances");
        instance_dir.push(&instance_name);
        instance_dir.push("data");
        let _ = std::fs::create_dir_all(&instance_dir);

        let mut cmd = Command::new(&self.config.java_path);
        cmd.current_dir(instance_dir);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        
        for arg in JAVA_ARGS {
            cmd.arg(arg);
        }
        
        cmd.arg(["-Xmx", &self.config.ram_amount.to_string(), "M"].concat());

        let mut natives_path = self.config.launcher_dir();
        natives_path.push("instances");
        natives_path.push(&instance_name);
        natives_path.push("natives");

        cmd.arg(["-Djava.library.path=", natives_path.to_str().unwrap() ].concat());
        cmd.arg(["-Dminecraft.client.jar=", client_jar.to_str().unwrap()].concat());
        cmd.arg("-cp");

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

                        let _ = zip_extract::extract(Cursor::new(data), &natives_path, true);
                    }
                } else {
                    let mut libs = self.config.launcher_dir();
                    libs.push("libraries");
                    libs.push(library.to_pathbuf_file(false));
                    if library.name.contains("com.mojang:authlib") {
                        if let Some(server) = special_server {
                            let mut patched_auth = self.config.launcher_dir();
                            patched_auth.push("libraries");
                            patched_auth.push(library.to_pathbuf_file(true));
                            let _ = nicotine::patch_jar(libs.to_str().unwrap(), patched_auth.to_str().unwrap(), &[if self.config.allow_http { "http://" } else { "https://" }, &server.domain, ":", &server.session_server_port.to_string(), "/api/"].concat());
                            libraries_cmd.push([patched_auth.to_str().unwrap(), ";"].concat());
                            println!("{:?}", patched_auth.to_str().unwrap());
                            continue;
                        }
                    }
                    libraries_cmd.push([libs.to_str().unwrap(), ";"].concat());
                }
            }
            libraries_cmd.push(client_jar.to_str().unwrap().to_string());
            cmd.arg(libraries_cmd.concat());
            println!("{:?}", libraries_cmd);
            cmd.arg(config.mainClass.clone());

            let mut game_dir = self.config.launcher_dir();
            game_dir.push("instances");
            game_dir.push(&instance_name);
            game_dir.push("data");

            let mut assets_dir = self.config.launcher_dir();
            assets_dir.push("assets");
            cmd.args(["--username", username, "--version", &instance_name, "--gameDir", game_dir.to_str().unwrap(), "--assetsDir", assets_dir.to_str().unwrap(), "--assetIndex", &config.assetIndex.id, "--uuid", &uuid, "--accessToken", &token, "--userProperties", "{}", "--userType", "mojang", "--width", "925", "--height", "530"]);
            if let Some(server) = special_server {
                cmd.arg("--server");
                cmd.arg(&server.domain);
                cmd.arg("--port");
                cmd.arg(server.port.to_string());
            }
            let mut child = cmd.spawn().unwrap();

            tokio::spawn(async move {
                
                if let Some(stdout) = child.stdout.take() {
                    if let Some(stderr) = child.stderr.take() {
                        let out_reader = BufReader::new(stdout);
                        let mut out_lines = out_reader.lines();

                        let err_reader = BufReader::new(stderr);
                        let mut err_lines = err_reader.lines();
                
                        loop {
                            tokio::select! {
                                Ok(Some(line)) = out_lines.next_line() => {
                                    let _ = sender.send(line);
                                }
                                Ok(Some(line)) = err_lines.next_line() => {
                                    let _ = sender.send(line);
                                }
                                else => break,
                            }
                        }
                        // end of minecraft launch
                    }
                }
            });
        }
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
                let mut dl_path = libraries.clone();
                let mut dl_pp = libraries.clone();
                dl_pp.push(library.to_pathbuf_path());
                let _ = std::fs::create_dir_all(dl_pp);
                dl_path.push(library.to_pathbuf_file(false));
                if File::open(dl_path.to_str().unwrap()).await.is_err() {
                    overall_size += artifact.size as usize;
                    let _ = util::download_file(&artifact.url, dl_path.to_str().unwrap(), sx.clone(), "Downloading libraries");
                    cnt += 1;
                }
            }

            if let Some(classifiers) = &library.downloads.classifiers {
                if let Some(natives) = &classifiers.natives {
                    let mut dl_path = libraries.clone();
                    dl_path.push(&natives.path);
                    let t_p = dl_path.to_str().unwrap().split("/").collect::<Vec<&str>>();
                    let t_p = t_p[..t_p.len()-1].join("/");
                    let _ = std::fs::create_dir_all(&t_p);
                    if File::open(dl_path.to_str().unwrap()).await.is_err() {
                        overall_size += natives.size as usize;
                        let _ = util::download_file(&natives.url, dl_path.to_str().unwrap(), sx.clone(), "Downloading natives");
                        cnt += 1;
                    }
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
            let _ = std::fs::create_dir_all(single_object_path);

            if File::open(single_object.to_str().unwrap()).await.is_err() {
                let _ = util::download_file(&asset.to_url(), single_object.to_str().unwrap(), sx.clone(), "Downloading assets objects");
                cnt += 1;
            }
        }

        tokio::spawn(async move {
            let mut current_size = 0;
            let mut current_cnt = 0;
            while let Some((size, status)) = rx.recv().await {
                current_size += size;
                current_cnt += 1;
                let _ = sender.send((((current_size as f32 / overall_size as f32) * 100.0) as u8, status));
                if current_cnt >= cnt {
                    let _ = sender.send((100, "_".to_string()));
                }
            }
        });
    }

    pub fn init_dirs(&self) {
        let root = self.config.launcher_dir();
        std::fs::create_dir_all(&root);
        // instances assets libraries config.toml
        let mut instances = root.clone();
        instances.push("instances");

        let mut assets = root.clone();
        assets.push("assets");

        let mut libraries = root.clone();
        libraries.push("libraries");

        std::fs::create_dir_all(&instances);
        std::fs::create_dir_all(&assets);
        std::fs::create_dir_all(&libraries);
    }
}