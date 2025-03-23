
pub mod versions {
    use std::{error::Error, path::PathBuf};

    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct VersionManifest {
        pub latest: Latest,
        pub versions: Vec<Version>
    }

    #[derive(Serialize, Deserialize)]
    pub struct Version {
        pub id: String, 
        pub r#type: String,
        pub url: String,
        pub time: String,
        pub releaseTime: String,
        pub sha1: String,
        pub complianceLevel: u8
    }

    #[derive(Serialize, Deserialize)]
    pub struct Latest {
        pub release: String,
        pub snapshot: String 
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct VersionConfig {
        pub assetIndex: Option<ConfigAssetIndex>,
        pub mainClass: String,
        pub minecraftArguments: String,
        pub downloads: Option<ConfigDownloads>,
        pub id: String,
        pub r#type: String,
        pub libraries: Vec<VersionLibrary>
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct VersionLibrary {
        pub downloads: LibraryDownloads,
        pub name: String,
    }

    impl VersionLibrary {
        pub fn to_pathbuf_path(&self) -> PathBuf {
            let mut p = PathBuf::new();
            let pkg = self.name.clone();
            let g = pkg.split(":").collect::<Vec<&str>>();
            let pkg_name = g[0];
            let artifact_name = g[1];
            let version = g[2];
            let b = pkg_name.split(".").collect::<Vec<&str>>();
            for h in b {
                p.push(h);
            }
            p.push(artifact_name);
            p.push(version);
            p
        }

        pub fn to_pathbuf_file(&self, is_patched: bool) -> PathBuf {
            let mut p = PathBuf::new();
            let pkg = self.name.clone();
            let g = pkg.split(":").collect::<Vec<&str>>();
            let pkg_name = g[0];
            let artifact_name = g[1];
            let version = g[2];
            let b = pkg_name.split(".").collect::<Vec<&str>>();
            for h in b {
                p.push(h);
            }
            p.push(artifact_name);
            p.push(version);
            if !is_patched {
                p.push([artifact_name, "-", version, ".jar"].concat());
            } else {
                p.push([artifact_name, "-", version, "-patch.jar"].concat());
            }
            p
        }
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct LibraryClassifiers {
        #[serde(rename = "natives-windows")]
        pub natives: Option<LibraryNatives>
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct LibraryNatives {
        pub path: String,
        pub sha1: String,
        pub size: u64,
        pub url: String
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct LibraryDownloads {
        pub artifact: Option<LibraryArtifact>,
        pub classifiers: Option<LibraryClassifiers>
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct LibraryArtifact {
        pub path: String,
        pub sha1: String, 
        pub size: u64,
        pub url: String
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct ConfigDownloads {
        pub client: ConfigDownloadsClient
    }
    
    #[derive(Serialize, Deserialize, Clone)]
    pub struct ConfigDownloadsClient {
        pub sha1: String,
        pub size: u64,
        pub url: String
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct ConfigAssetIndex {
        pub id: String,
        pub sha1: String,
        pub totalSize: u64,
        pub size: u64,
        pub url: String
    }

    impl ConfigAssetIndex {
        pub fn to_path(&self) -> PathBuf {
            let mut p = PathBuf::new();
            p.push("indexes");
            p.push([&self.id, ".json"].concat());
            p
        }
    }

    pub async fn fetch_versions_list() -> Result<VersionManifest, Box<dyn Error + Send + Sync>> {
        let mut r = surf::get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json").await?;
        let resp = r.body_bytes().await.unwrap();
        let m = serde_json::from_slice(&resp)?;
        Ok(m)
    }

    pub async fn fetch_version_object(version: &Version) -> Result<VersionConfig, Box<dyn Error + Send + Sync>> {
        let url = version.url.clone();
        let mut r = surf::get(url).await?;
        let resp = r.body_bytes().await.unwrap();
        let resp: VersionConfig = serde_json::from_slice(&resp)?;
        Ok(resp)
    }

    pub async fn find_version_object(version: &str) -> Result<VersionConfig, Box<dyn Error + Send + Sync>> {
        let versions = fetch_versions_list().await?;
        let versions = versions.versions;
        let version = versions.iter().find(|v| v.id == version).unwrap();
        let config = fetch_version_object(version).await?;
        Ok(config)
    }
}

pub mod session {
    use std::error::Error;

    use serde::{Deserialize, Serialize};

    #[derive(Serialize)]
    struct SignUpRequest {
        username: String,
        password: String,
    }

    pub enum SignUpResponse {
        Registered(String),
        BadCredentials,
        UserAlreadyExists,
        ServerError
    }

    #[derive(Deserialize)]
    struct ResponseUUID {
        uuid: String
    }

    pub async fn try_signup(server_domain: String, port: u16, username: String, password: String, allow_http: bool) -> Result<SignUpResponse, Box<dyn Error + Send + Sync>> {
        let request = SignUpRequest { username, password };
        let mut r = surf::post([if allow_http { "http://".to_string() } else { "https://".to_string() }, server_domain, ":".to_string(), port.to_string(), "/api/register".to_string()].concat())
            .body_json(&request)
            .unwrap()
            .await?;

        let b=  r.body_bytes().await.unwrap();
        match r.status() {
            surf::StatusCode::BadRequest => Ok(SignUpResponse::BadCredentials),
            surf::StatusCode::Conflict => Ok(SignUpResponse::UserAlreadyExists),
            surf::StatusCode::Ok => {
                let response: ResponseUUID = serde_json::from_slice(&b).unwrap();
                Ok(SignUpResponse::Registered(response.uuid))
            },
            _ => Ok(SignUpResponse::ServerError)
        }
    }

    pub async fn try_login(server_domain: String, port: u16, username: String, password: String, allow_http: bool) -> Result<SignUpResponse, Box<dyn Error + Send + Sync>> {
        let request = SignUpRequest { username: username.clone(), password };
        let mut r = surf::post([if allow_http { "http://".to_string() } else { "https://".to_string() }, server_domain, ":".to_string(), port.to_string(), "/api/login".to_string()].concat())
            .body_json(&request)
            .unwrap()
            .await?;

        let b=  r.body_bytes().await.unwrap();

        match r.status() {
            surf::StatusCode::BadRequest => Ok(SignUpResponse::BadCredentials),
            surf::StatusCode::Conflict => Ok(SignUpResponse::UserAlreadyExists),
            surf::StatusCode::Ok => {
                let response: ResponseUUID = serde_json::from_slice(&b).unwrap();
                Ok(SignUpResponse::Registered(response.uuid))
            },
            _ => Ok(SignUpResponse::ServerError)
        }
    }
}

pub mod multimc {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct Pack {
        pub components: Vec<Component>
    }

    #[derive(Serialize, Deserialize)]
    pub struct Component {
        pub cachedName: Option<String>,
        pub version: String,
        pub uid: String,
    }
}

pub mod server {
    use std::error::Error;

    use base64::{prelude::BASE64_STANDARD, Engine};
    use surf::StatusCode;

    pub async fn get_server_icon(server: &str, port: u16) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        let mut r = surf::get(["https://eu.mc-api.net/v3/server/favicon/", server,":", &port.to_string()].concat()).await?;
        if r.status() != StatusCode::Ok { return Ok(None); }
        let resp = r.body_bytes().await.unwrap();
        Ok(Some(["data:image/png;base64,", &BASE64_STANDARD.encode(resp)].concat()))
    }
}

pub mod assets {
    use std::{collections::HashMap, error::Error, path::PathBuf};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct SingleAsset {
        pub hash: String,
        pub sha1: Option<String>
    }

    impl SingleAsset {
        pub fn to_url(&self) -> String {
            ["https://resources.download.minecraft.net/", &self.hash[..2], "/", &self.hash].concat()
        }

        pub fn to_path(&self) -> PathBuf {
            let mut p = PathBuf::new();
            p.push("objects");
            p.push(&self.hash[..2]);
            p.push(&self.hash);
            p
        }

        pub fn to_small_path(&self) -> PathBuf {
            let mut p = PathBuf::new();
            p.push("objects");
            p.push(&self.hash[..2]);
            p
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct Assets {
        pub objects: HashMap<String, SingleAsset>
    }

    pub async fn fetch_assets_list(url: &str) -> Result<Assets, Box<dyn Error + Send + Sync>> {
        let mut r = surf::get(url).await?;
        let resp = r.body_bytes().await.unwrap();
        let resp: Assets = serde_json::from_slice(&resp)?;
        Ok(resp)
    }
}