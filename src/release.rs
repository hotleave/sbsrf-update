use regex::Regex;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Tag {
    name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AttachFile {
    pub name: String,
    pub download_url: String,
}

#[derive(Deserialize, Debug, Clone)]
struct ReleaseDetail {
    title: String,
    created_at: String,
    description: String,
    attach_files: Vec<AttachFile>,
}

#[derive(Deserialize, Debug)]
pub struct ReleaseBase {
    release: ReleaseDetail,
    tag: Tag,
}

#[derive(Deserialize, Debug)]
pub struct Release {
    release: ReleaseBase,
}

impl Release {
    pub async fn init() -> Result<Self, reqwest::Error> {
        let response = reqwest::get("http://127.0.0.1:18080/sbxlm/sbxlm/releases/latest").await?;
        // let response = reqwest::get("https://gitee.com/sbxlm/sbxlm/releases/latest").await?;
        response.json::<Release>().await
    }

    pub fn get_version(&self) -> String {
        self.release.tag.name.clone()
    }

    pub fn get_assets(&self) -> Vec<AttachFile> {
        self.release.release.attach_files.clone()
    }

    pub fn get_download_url(&self, uri: String) -> String {
        // let url = format!("https://gitee.com{}", uri);
        format!("http://127.0.0.1:18080{}", &uri)
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
