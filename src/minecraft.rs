
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

    #[derive(Serialize, Deserialize)]
    pub struct VersionConfig {
        pub assetIndex: ConfigAssetIndex,
        pub mainClass: String,
        pub downloads: ConfigDownloads,
        pub id: String,
        pub libraries: Vec<VersionLibrary>
    }

    #[derive(Serialize, Deserialize)]
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

        pub fn to_pathbuf_file(&self) -> PathBuf {
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
            p.push(vec![artifact_name, "-", version, ".jar"].concat());
            p
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct LibraryDownloads {
        pub artifact: Option<LibraryArtifact>
    }

    #[derive(Serialize, Deserialize)]
    pub struct LibraryArtifact {
        pub path: String,
        pub sha1: String, 
        pub size: u64,
        pub url: String
    }

    #[derive(Serialize, Deserialize)]
    pub struct ConfigDownloads {
        pub client: ConfigDownloadsClient
    }
    
    #[derive(Serialize, Deserialize)]
    pub struct ConfigDownloadsClient {
        pub sha1: String,
        pub size: u64,
        pub url: String
    }

    #[derive(Serialize, Deserialize)]
    pub struct ConfigAssetIndex {
        pub id: String,
        pub sha1: String,
        pub totalSize: u64,
        pub url: String
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
}