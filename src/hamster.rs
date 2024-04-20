use std::{fs, path::{Path, PathBuf}};

use indicatif::{MultiProgress, ProgressBar};
use tempfile::tempdir;

use crate::{
    im::{check_file_item, IMUpdateConfig, InputMethod},
    utils::{
        download_and_install, download_file, get_bar_style, get_spinner_style, unzip,
        upload_to_ios, work_dir,
    },
};

#[derive(Debug)]
pub struct Hamster {
    pub config: IMUpdateConfig,
    pub host: String,
}

impl Hamster {
    pub fn new(config: IMUpdateConfig, host: String) -> Self {
        Self { config, host }
    }

    pub fn default_config(name: &str) -> IMUpdateConfig {
        let update_dir = work_dir().join(name);

        IMUpdateConfig {
            name: "Hamster".to_string(),
            exe: None,
            user_dir: PathBuf::new(),
            update_dir,
            max_backups: 1,
            sentence: false,
            version: "20051203".to_string(),
        }
    }
}

async fn download_and_upload_to_ios(
    name: String,
    download_url: String,
    device_host: String,
    m: MultiProgress,
) {
    let target_dir = tempdir().unwrap().into_path();
    download_and_install(target_dir.clone(), name, download_url, m.clone()).await;

    let pb = m.add(ProgressBar::new_spinner());
    pb.set_style(get_spinner_style());
    pb.set_prefix("上传");
    upload_to_ios(&target_dir, &device_host, &pb).await.unwrap();
    pb.finish_with_message("完成");
}

impl InputMethod for Hamster {
    async fn install(&self, _: &str, _: &str) {
        todo!()
    }

    async fn backup(&self) {
        let pb = ProgressBar::new(100);
        pb.set_style(get_bar_style());
        pb.set_prefix("备份");

        let url = format!("http://{}/api/raw/Rime", &self.host);
        let target_path = self
            .config
            .update_dir
            .join("backups")
            .join(&self.config.version);
        if !target_path.exists() {
            fs::create_dir_all(&target_path).unwrap();
        }

        if let Err(err) = download_file(url, &target_path.join("Rime.zip"), |len, total| {
            pb.set_length(total);
            pb.inc(len as u64);
        })
        .await
        {
            eprintln!("备份失败: {}", err);
        }
        pb.finish();
    }

    async fn restore(&self, version: &Path) {
        // 解压
        let file_path = version.join("Rime.zip");
        let output_dir = work_dir().join("_cache");
        let pb = ProgressBar::new_spinner();
        pb.set_style(get_spinner_style());
        pb.set_prefix("解压");
        unzip(&file_path, &output_dir, &pb).await;

        let pb = ProgressBar::new_spinner();
        pb.set_style(get_spinner_style());
        pb.set_prefix("上传");
        let from = output_dir.join("Rime");
        upload_to_ios(&from, &self.host, &pb).await.unwrap();
        fs::remove_dir_all(from).unwrap();

        println!("还原完成，需要在手机上重新部署");
    }

    async fn update(&self, release: crate::release::Release) {
        println!("开始为本地的鼠须管更新声笔输入法...");
        self.backup().await;

        let assets = release.get_assets();
        let m = MultiProgress::new();
        let mut tasks = vec![];
        let host = self.host.clone();

        for asset in assets {
            if !check_file_item(&asset.name, "hamster", self.config.sentence) {
                continue;
            }

            let name = asset.name;
            let download_url = release.get_download_url(asset.download_url);
            let task = tokio::spawn(download_and_upload_to_ios(
                name,
                download_url,
                host.clone(),
                m.clone(),
            ));
            tasks.push(task);
        }

        for task in tasks {
            if let Err(error) = task.await {
                println!("更新失败：{error}");
            }
        }

        println!("更新完成，需要在手机上重新部署");
    }

    fn deploy(&self) {
        todo!()
    }
}
