
pub mod versions {
    use std::error::Error;

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

    #[derive(Serialize, Deserialize)]
    pub struct LibraryDownloads {
        pub artifact: LibraryArtifact
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
        let resp: VersionManifest = reqwest::get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
            .await?
            .json()
            .await?;

        Ok(resp)
    }

    pub async fn fetch_version_object(version: &Version) -> Result<VersionConfig, Box<dyn Error + Send + Sync>> {
        let resp: String = reqwest::get(&version.url)
        .await?
        .text()
        .await?;

        let resp: VersionConfig = serde_json::from_str(&resp)?;
        Ok(resp)
    }
}