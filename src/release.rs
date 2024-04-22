use regex::Regex;
use reqwest::Method;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Asset {
    pub name: String,

    #[serde(rename = "browser_download_url")]
    pub download_url: String,
}

impl Asset {
    pub fn new(name: String, download_url: String) -> Self {
        Self { name, download_url }
    }
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
    pub async fn init(use_github: bool) -> Result<Self, reqwest::Error> {
        if use_github {
            GithubRelease::init().await
        } else {
            GiteeRelease::init().await
        }
    }
}

#[derive(Deserialize, Debug)]
struct GiteeTag {
    name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GiteeAttachFile {
    pub name: String,
    pub download_url: String,
}

#[derive(Deserialize, Debug, Clone)]
struct GiteeReleaseDetail {
    title: String,
    created_at: String,
    description: String,
    attach_files: Vec<GiteeAttachFile>,
}

#[derive(Deserialize, Debug)]
pub struct GiteeReleaseBase {
    release: GiteeReleaseDetail,
    tag: GiteeTag,
}

#[derive(Deserialize, Debug)]
pub struct GiteeRelease {
    release: GiteeReleaseBase,
}

impl GiteeRelease {
    pub async fn init() -> Result<Release, reqwest::Error> {
        // let response = reqwest::get("http://127.0.0.1:18080/sbxlm/sbxlm/releases/latest").await?;
        let response = reqwest::get("https://gitee.com/sbxlm/sbxlm/releases/latest").await?;
        match response.json::<GiteeRelease>().await {
            Ok(gitee) => {
                let version = gitee.get_version();
                let intro = gitee.get_release_info();
                let assets: Vec<Asset> = gitee
                    .get_assets()
                    .iter()
                    .map(|x| {
                        Asset::new(
                            x.name.clone(),
                            format!("https://gitee.com{}", x.download_url),
                        )
                    })
                    .collect();

                Ok(Release {
                    version,
                    intro,
                    assets,
                })
            }
            Err(error) => Err(error),
        }
    }

    pub fn get_version(&self) -> String {
        self.release.tag.name.clone()
    }

    pub fn get_assets(&self) -> Vec<GiteeAttachFile> {
        self.release.release.attach_files.clone()
    }

    pub fn get_release_info(&self) -> String {
        let release = self.release.release.clone();
        let re = Regex::new(r"</?p>|<br/?>").unwrap();
        let description = re.replace_all(release.description.as_str(), "");

        format!(
            "{title}\n\n{release_at}\n\n{description}",
            title = release.title,
            release_at = release.created_at,
            description = description
        )
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
