use regex::Regex;
use serde::Deserialize;

#[derive(Deserialize)]
#[derive(Debug)]
struct Commit {
  id: String,
}

#[derive(Deserialize)]
#[derive(Debug)]
struct Tag {
  name: String,
  commit: Commit,
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

#[derive(Deserialize)]
#[derive(Debug)]
pub struct ReleaseBase {
  release: ReleaseDetail,
  tag: Tag,
}

#[derive(Deserialize)]
#[derive(Debug)]
pub struct Release {
  release: ReleaseBase,
}

impl Release {
  pub async fn init() -> Result<Self, reqwest::Error> {
    // let response = reqwest::get("http://127.0.0.1:18080/sbxlm/sbxlm/releases/latest").await?;
    let response = reqwest::get("https://gitee.com/sbxlm/sbxlm/releases/latest").await?;
    Ok(response.json::<Release>().await?)
  }

  pub fn get_version(&self) -> String {
    self.release.tag.name.clone()
  }
  
  pub fn get_id(&self) -> String {
    self.release.tag.commit.id.clone()
  }
  pub fn get_assets(&self) -> Vec<AttachFile> {
    self.release.release.attach_files.clone()
  }

  pub fn get_release_info(&self) -> String {
    let release = self.release.release.clone();
    let re = Regex::new(r"</?p>|<br/?>").unwrap();
    let description = re.replace_all(release.description.as_str(), "");

    return format!("{title}\n\n{release_at}\n\n{description}", title = release.title, release_at = release.created_at, description = description)
  }
}

