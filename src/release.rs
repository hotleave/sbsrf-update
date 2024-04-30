use reqwest::Method;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Asset {
    pub name: String,

    #[serde(rename = "browser_download_url")]
    pub download_url: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Release {
    #[serde(rename = "tag_name")]
    pub version: String,

    #[serde(rename = "body")]
    pub intro: String,
    pub assets: Vec<Asset>,
}

impl Release {
    pub async fn init() -> Result<Self, reqwest::Error> {
        GithubRelease::init().await
    }
}
pub struct GithubRelease {}

impl GithubRelease {
    pub async fn init() -> Result<Release, reqwest::Error> {
        let response = reqwest::Client::new()
            .request(
                Method::GET,
                "https://api.github.com/repos/sbsrf/home/releases/latest",
            )
            .header("User-Agent", "Sbsrf-Update-App")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?;
        response.json::<Release>().await
    }
}
