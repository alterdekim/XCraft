use core::str;
use std::error::Error;
use std::io::Cursor;
use std::path::PathBuf;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use rand::rngs::StdRng;
use rand::seq::IndexedRandom;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use surf::StatusCode;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use crate::config::{LauncherCredentials, LauncherServer};
use crate::minecraft;
use crate::minecraft::multimc::Pack;
use crate::minecraft::session::SignUpResponse;
use crate::minecraft::versions::Version;
use crate::{config::LauncherConfig, minecraft::versions::VersionConfig, util};
use ureq_multipart::MultipartBuilder;

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

    pub async fn upload_skin(&self, file_path: PathBuf, uuid: &str, password: &str, server_url: &str) -> Result<String, Box<dyn Error + Sync + Send>> {
        let (content_type,data) = MultipartBuilder::new()
            .add_file("skin",file_path)?
            .finish()?;

        let mut resp = ureq::post(server_url)
                    .content_type(content_type)
                    .query_pairs(vec![("uuid", uuid), ("password", password)])
                    .send(data)?;

        let s = resp.body_mut().read_to_string()?;

        Ok(s)
    }

    pub async fn upload_cape(&self, file_path: PathBuf, uuid: &str, password: &str, server_url: &str) -> Result<String, Box<dyn Error + Sync + Send>> {
        let (content_type,data) = MultipartBuilder::new()
            .add_file("cape",file_path)?
            .finish()?;

        let mut resp = ureq::post(server_url)
                    .content_type(content_type)
                    .query_pairs(vec![("uuid", uuid), ("password", password)])
                    .send(data)?;

        let s = resp.body_mut().read_to_string()?;

        Ok(s)
    }

    pub async fn set_skin_model(&self, is_slim: bool, uuid: &str, password: &str, server_url: &str) -> Result<String, Box<dyn Error + Sync + Send>> {
        let mut resp = ureq::post(server_url)
                    .query_pairs(vec![("uuid", uuid), ("password", password), ("model", &is_slim.to_string())])
                    .send_empty()?;

        let s = resp.body_mut().read_to_string()?;

        Ok(s)
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

    pub async fn login_user_server(&mut self, server: String, username: String, password: String) -> (bool, &str) {
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

        match minecraft::session::try_login(domain.clone(), session_server_port, username.clone(), password.clone(), self.config.allow_http).await {
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

    pub fn find_credentials(&self, username: &str, domain: &str) -> Option<&LauncherServer> {
        let servers = self.config.servers();
        servers.iter().find(|&server| server.domain == domain && server.credentials.username == username)
    }

    pub fn get_instances_list(&self) -> Vec<(String, String, String)> {
        let mut v = Vec::new();
        let instances = self.config.instances_path();
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
                    v.push((entry.file_name().into_string().unwrap(), c_type.to_string(),  format!("data:image/png;base64,{}", BASE64_STANDARD.encode(match c_type {
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
        let instances = self.config.instances_path();
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
        
        let mut instances = self.config.instances_path();
        instances.push(&instance_name);
        instances.push("client.json");

        let mut client_jar = self.config.instances_path();
        client_jar.push(&instance_name);
        client_jar.push("client.jar");

        let mut instance_dir = self.config.instances_path();
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

        let mut natives_path = self.config.instances_path();
        natives_path.push(&instance_name);
        natives_path.push("natives");

        cmd.arg(["-Djava.library.path=", natives_path.to_str().unwrap() ].concat());
        cmd.arg(["-Dminecraft.client.jar=", client_jar.to_str().unwrap()].concat());
        cmd.arg("-cp");

        let mut minecraft_arguments = None;

        if let Ok(data) = std::fs::read(&instances) {
            let config: VersionConfig = serde_json::from_slice(&data).unwrap();
            minecraft_arguments = Some(config.minecraft_arguments);
            let mut libraries_cmd = Vec::new();
            for library in config.libraries {
                if let Some(classifier) = &library.downloads.classifiers {
                    if let Some(natives) = &classifier.natives {
                        let rel_path = &natives.path;
                        let libs = self.config.libraries_path();
                        let rel_path = [libs.to_str().unwrap(), "\\", &rel_path.replace("/", "\\")].concat();
                        let data = std::fs::read(rel_path).unwrap();

                        let _ = zip_extract::extract(Cursor::new(data), &natives_path, true);
                    }
                } else {
                    let mut libs = self.config.libraries_path();
                    libs.push(library.to_pathbuf_file(false));
                    if library.name.contains("com.mojang:authlib") {
                        if let Some(server) = special_server {
                            let mut patched_auth = self.config.libraries_path();
                            patched_auth.push(library.to_pathbuf_file(true));
                            let _ = nicotine::patch_jar(libs.to_str().unwrap(), patched_auth.to_str().unwrap(), [b"https://sessionserver.mojang.com/session/minecraft/".as_slice(), b".minecraft.net".as_slice()].as_slice(),  &[&[if self.config.allow_http { "http://" } else { "https://" }, &server.domain, ":", &server.session_server_port.to_string(), "/api/"].concat(), &server.domain]);
                            libraries_cmd.push([patched_auth.to_str().unwrap(), ";"].concat());
                            continue;
                        }
                    }
                    libraries_cmd.push([libs.to_str().unwrap(), ";"].concat());
                }
            }
            libraries_cmd.push(client_jar.to_str().unwrap().to_string());
            cmd.arg(libraries_cmd.concat());
            cmd.arg(config.main_class.clone());

            let mut game_dir = self.config.instances_path();
            game_dir.push(&instance_name);
            game_dir.push("data");

            let mut assets_dir = self.config.assets_path();

            let minecraft_arguments = minecraft_arguments.unwrap();
            let minecraft_arguments = minecraft_arguments.split(" ");
            for minecraft_argument in minecraft_arguments {
                cmd.arg(match minecraft_argument {
                    "${auth_player_name}" => username,
                    "${version_name}" => &instance_name,
                    "${game_directory}" => game_dir.to_str().unwrap(),
                    "${assets_root}" => assets_dir.to_str().unwrap(),
                    "${assets_index_name}" => &config.asset_index.as_ref().unwrap().id,
                    "${auth_uuid}" => &uuid,
                    "${auth_access_token}" => &token,
                    "${user_properties}" => "{}",
                    "${user_type}" => "mojang",
                    "${version_type}" => "modified",
                    _ => minecraft_argument
                });
            }

            //cmd.args(["--username", username, "--version", &instance_name, "--gameDir", game_dir.to_str().unwrap(), "--assetsDir", assets_dir.to_str().unwrap(), "--assetIndex", &config.assetIndex.id, "--uuid", &uuid, "--accessToken", &token, "--userProperties", "{}", "--userType", "mojang", "--width", "925", "--height", "530"]);
            assets_dir.push("skins");
            let _ = std::fs::remove_dir_all(assets_dir);
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


    pub async fn import_multimc(&self, instance_path: PathBuf, sender: UnboundedSender<(u8, String)>) -> Result<(), Box<dyn Error + Send + Sync>> {
        let (sx, mut rx) = mpsc::unbounded_channel();


        let instance_name = instance_path.file_name().unwrap().to_str().unwrap();
        let instance_name = [&instance_name[..instance_name.len()-4], "_", &util::random_string(4)].concat();
        let mut instance_dir = self.config.instances_path();
        instance_dir.push(&instance_name);
        let _ = std::fs::create_dir_all(&instance_dir);
        let data = std::fs::read(instance_path)?;
        zip_extract::extract(Cursor::new(data), &instance_dir, true)?;
        let mut multimc_data = instance_dir.clone();
        multimc_data.push(".minecraft");
        let mut data_dir = instance_dir.clone();
        data_dir.push("data");
        std::fs::rename(multimc_data, data_dir)?;

        let mut pack_path = instance_dir.clone();
        pack_path.push("mmc-pack.json");
        println!("pack_path {}", pack_path.to_str().unwrap());
        let multimc_data = std::fs::read(pack_path)?;
        let pack_mmc: Pack = serde_json::from_slice(&multimc_data)?;

        let mut minecraft_config = None;
        let mut forge_version = None;

        for component in pack_mmc.components.iter().filter(|c| c.cached_name.is_some()) {
            match component.cached_name.as_ref().unwrap().as_str() {
                "Minecraft" => minecraft_config = Some(crate::minecraft::versions::find_version_object(&component.version).await?),
                "Forge" => {
                    forge_version = Some(component.version.clone());
                }
                _ => {}
            }
        }

        let mut overall_size = 0;
        let mut cnt = 0;

        let mut client_json_path = self.config.instances_path();
        client_json_path.push(&instance_name);
        client_json_path.push("client.json");

        if minecraft_config.is_some() {
            let config = minecraft_config.clone().unwrap();
            let config_cl = config.clone();
            
            overall_size = config.downloads.as_ref().unwrap().client.size as usize;
            cnt = 0;

            let client_jar_url = config.downloads.as_ref().unwrap().client.url.clone();

            let mut client_jar_path = self.config.instances_path();
            client_jar_path.push(&instance_name);
            client_jar_path.push("client.jar");

            std::fs::write(&client_json_path, serde_json::to_string_pretty(&config_cl).unwrap())?;

            let _ = util::download_file(&client_jar_url, client_jar_path.to_str().unwrap(), sx.clone(), "Downloading client.jar", false).await;
            cnt += 1;

            let libraries = self.config.libraries_path();

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
                        let _ = util::download_file(&artifact.url, dl_path.to_str().unwrap(), sx.clone(), "Downloading libraries", false).await;
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
                            let _ = util::download_file(&natives.url, dl_path.to_str().unwrap(), sx.clone(), "Downloading natives", false).await;
                            cnt += 1;
                        }
                    }
                }
            }

            let assets_path = self.config.assets_path();

            let mut indexes = assets_path.clone();
            indexes.push("indexes");
            let _ = std::fs::create_dir_all(indexes);

            let mut objects = assets_path.clone();
            objects.push("objects");
            let _ = std::fs::create_dir_all(objects);

            let mut index = assets_path.clone();
            index.push(config.asset_index.as_ref().unwrap().to_path());

            let _ = util::download_file(&config.asset_index.as_ref().unwrap().url, index.to_str().unwrap(), sx.clone(), "Downloading assets indexes", false).await;
            cnt += 1;

            let asset_index = config.asset_index.as_ref().unwrap().url.clone();

            overall_size += config.asset_index.as_ref().unwrap().size as usize;
            overall_size += config.asset_index.as_ref().unwrap().total_size as usize;

            let assets = crate::minecraft::assets::fetch_assets_list(&asset_index).await.unwrap().objects;

            for (_key, asset) in assets {
                let mut single_object = assets_path.clone();
                single_object.push(asset.to_path());

                let mut single_object_path = assets_path.clone();
                single_object_path.push(asset.to_small_path());
                let _ = std::fs::create_dir_all(single_object_path);

                if File::open(single_object.to_str().unwrap()).await.is_err() {
                    let _ = util::download_file(&asset.to_url(), single_object.to_str().unwrap(), sx.clone(), "Downloading assets objects", false).await;
                    cnt += 1;
                }
            }
        }

        if minecraft_config.is_some() && forge_version.is_some() {
            let mut forge_installer_path = self.config.libraries_path();
            forge_installer_path.push("forge_installer.jar");

            let mut forge_installer_unpack = self.config.libraries_path();
            forge_installer_unpack.push("installer_unpacked");

            std::fs::create_dir_all(&forge_installer_unpack)?;

            let forge_installer_url = format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{}-{}/forge-{}-{}-installer.jar", minecraft_config.as_ref().unwrap().id, forge_version.as_ref().unwrap(), minecraft_config.as_ref().unwrap().id, forge_version.as_ref().unwrap());

            let _ = util::download_file(&forge_installer_url, forge_installer_path.to_str().unwrap(), sx.clone(), "Downloading forge installer", true).await;
            cnt += 1;

            let forge_installer_data = std::fs::read(&forge_installer_path)?;

            zip_extract::extract(Cursor::new(forge_installer_data), &forge_installer_unpack, true)?;

            let mut forge_library_path = self.config.libraries_path();
            forge_library_path.push("net");
            forge_library_path.push("minecraftforge");
            forge_library_path.push("forge");
            forge_library_path.push(format!("{}-{}", minecraft_config.as_ref().unwrap().id, forge_version.as_ref().unwrap()));

            let mut forge_version_json = forge_installer_unpack.clone();
            forge_version_json.push("version.json");

            let mut forge_installer_library = forge_installer_unpack.clone();
            forge_installer_library.push("maven");
            forge_installer_library.push("net");
            forge_installer_library.push("minecraftforge");
            forge_installer_library.push("forge");
            forge_installer_library.push(format!("{}-{}", minecraft_config.as_ref().unwrap().id, forge_version.as_ref().unwrap()));
            forge_installer_library.push(format!("forge-{}-{}.jar", minecraft_config.as_ref().unwrap().id, forge_version.as_ref().unwrap()));

            std::fs::create_dir_all(&forge_library_path)?;

            forge_library_path.push(format!("forge-{}-{}.jar", minecraft_config.as_ref().unwrap().id, forge_version.as_ref().unwrap()));

            std::fs::copy(&forge_installer_library, &forge_library_path)?;

            let version_json = std::fs::read(&forge_version_json)?;
            let version_json: VersionConfig = serde_json::from_slice(&version_json)?;

            let mut edited = minecraft_config.clone().unwrap();
            edited.main_class = version_json.main_class.clone();
            edited.minecraft_arguments = version_json.minecraft_arguments;
            edited.libraries.retain(|l| !version_json.libraries.iter().any(|t| t.name == l.name));
            for i in 0..version_json.libraries.len() {
                edited.libraries.push(version_json.libraries[i].clone());
            }

            std::fs::write(&client_json_path, serde_json::to_string_pretty(&edited).unwrap())?;

            std::fs::remove_dir_all(forge_installer_unpack)?;
            std::fs::remove_file(forge_installer_path)?;

            let libraries = self.config.libraries_path();

            for i in 0..edited.libraries.len() {
                let library = &edited.libraries[i];
                if let Some(artifact) = &library.downloads.artifact {
                    let mut dl_path = libraries.clone();
                    let mut dl_pp = libraries.clone();
                    dl_pp.push(library.to_pathbuf_path());
                    let _ = std::fs::create_dir_all(dl_pp);
                    dl_path.push(library.to_pathbuf_file(false));
                    if File::open(dl_path.to_str().unwrap()).await.is_err() {
                        overall_size += artifact.size as usize;
                        let _ = util::download_file(&artifact.url, dl_path.to_str().unwrap(), sx.clone(), "Downloading libraries", false).await;
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
                            let _ = util::download_file(&natives.url, dl_path.to_str().unwrap(), sx.clone(), "Downloading natives", false).await;
                            cnt += 1;
                        }
                    }
                }
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

        Ok(())
    }

    pub async fn new_vanilla_instance(&mut self, config: VersionConfig, version_object: &Version, sender: UnboundedSender<(u8, String)>) {
        
        let (sx, mut rx) = mpsc::unbounded_channel();
        
        let mut instances = self.config.instances_path();
        instances.push(&config.id);

        let _ = std::fs::create_dir_all(&instances);

        instances.push("client.jar");

        let mut overall_size = config.downloads.as_ref().unwrap().client.size as usize;
        let mut cnt = 0;

        let client_jar_url = config.downloads.as_ref().unwrap().client.url.clone();

        let mut client_json_path = self.config.instances_path();
        client_json_path.push(config.id);
        client_json_path.push("client.json");

        let _ = util::download_file(&version_object.url, client_json_path.to_str().unwrap(), sx.clone(), "Downloading client.json", false).await;
        cnt += 1;

        let _ = util::download_file(&client_jar_url, instances.to_str().unwrap(), sx.clone(), "Downloading client.jar", false).await;
        cnt += 1;

        let libraries = self.config.libraries_path();

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
                    let _ = util::download_file(&artifact.url, dl_path.to_str().unwrap(), sx.clone(), "Downloading libraries", false).await;
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
                        let _ = util::download_file(&natives.url, dl_path.to_str().unwrap(), sx.clone(), "Downloading natives", false).await;
                        cnt += 1;
                    }
                }
            }
        }

        let assets_path = self.config.assets_path();

        let mut indexes = assets_path.clone();
        indexes.push("indexes");
        let _ = std::fs::create_dir_all(indexes);

        let mut objects = assets_path.clone();
        objects.push("objects");
        let _ = std::fs::create_dir_all(objects);

        let mut index = assets_path.clone();
        index.push(config.asset_index.as_ref().unwrap().to_path());

        let _ = util::download_file(&config.asset_index.as_ref().unwrap().url.clone(), index.to_str().unwrap(), sx.clone(), "Downloading assets indexes", false).await;
        cnt += 1;

        let asset_index = config.asset_index.as_ref().unwrap().url.clone();

        overall_size += config.asset_index.as_ref().unwrap().size as usize;
        overall_size += config.asset_index.as_ref().unwrap().total_size as usize;

        let assets = crate::minecraft::assets::fetch_assets_list(&asset_index).await.unwrap().objects;

        for (_key, asset) in assets {
            let mut single_object = assets_path.clone();
            single_object.push(asset.to_path());

            let mut single_object_path = assets_path.clone();
            single_object_path.push(asset.to_small_path());
            let _ = std::fs::create_dir_all(single_object_path);

            if File::open(single_object.to_str().unwrap()).await.is_err() {
                let _ = util::download_file(&asset.to_url(), single_object.to_str().unwrap(), sx.clone(), "Downloading assets objects", false).await;
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
        let _ = std::fs::create_dir_all(&root);
        // instances assets libraries config.toml
        let instances = self.config.instances_path();
        let assets = self.config.assets_path();
        let libraries = self.config.libraries_path();

        let _ = std::fs::create_dir_all(&instances);
        let _ = std::fs::create_dir_all(&assets);
        let _ = std::fs::create_dir_all(&libraries);
    }
}

#[derive(Serialize, Deserialize)]
struct BackgroundFiles {
    name: String
}

pub async fn get_random_bg() -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
    let mut r = surf::get("https://minecraft.awain.net/xcraft/").await?;
    if r.status() != StatusCode::Ok { return Ok(None); }
    let resp = r.body_bytes().await?;
    let resp: Vec<BackgroundFiles> = serde_json::from_slice(&resp)?;
    let mut rng = StdRng::from_os_rng();
    if let Some(resp) = resp.choose(&mut rng) {
        let mut r = surf::get(["https://minecraft.awain.net/xcraft/", &resp.name].concat()).await?;
        if r.status() != StatusCode::Ok { return Ok(None); }
        let resp = r.body_bytes().await?;
        return Ok(Some(["data:image/jpeg;base64,", &BASE64_STANDARD.encode(resp)].concat()));
    }
    Ok(None)
}