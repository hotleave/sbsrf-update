use std::{
    env::consts::OS,
    fs::{self, File},
    io::{copy, Cursor},
    path::{Path, PathBuf},
    process::Command,
};

use indicatif::{MultiProgress, ProgressBar};
use std::io::prelude::*;
use tempfile::tempdir;
use zip::ZipArchive;

use crate::{
    im::{check_file_item, IMUpdateConfig, InputMethod},
    release::Release,
    utils::{
        copy_dir_contents, download_and_install, download_file, ensure_max_backups, get_bar_style,
        get_spinner_style, grep, open, work_dir,
    },
};

#[derive(Debug)]
pub struct Squirrel {
    pub config: IMUpdateConfig,
}

impl Squirrel {
    pub fn new(config: IMUpdateConfig) -> Self {
        Self { config }
    }

    pub fn default_config() -> IMUpdateConfig {
        let update_dir = work_dir().join(OS);
        IMUpdateConfig {
            name: "Squirrel".to_string(),
            exe: Some(PathBuf::from(
                "/Library/Input Methods/Squirrel.app/Contents/MacOS/Squirrel",
            )),
            user_dir: PathBuf::from(std::env::var("HOME").unwrap()).join("Library/Rime"),
            update_dir,
            max_backups: 1,
            sentence: false,
            version: "20051203".to_string(),
        }
    }
}

impl InputMethod for Squirrel {
    async fn install(&self, name: &str, download_url: &str) {
        let file_path = work_dir().join("_cache").join(name);
        let pb = ProgressBar::new(100);
        pb.set_prefix(format!("下载 {}", name));
        pb.set_style(get_bar_style());
        if let Err(error) = download_file(download_url.to_string(), &file_path, |len, total| {
            pb.set_length(total);
            pb.inc(len as u64);
        })
        .await
        {
            println!("下载文件{}失败: {error}", name);
        }
        pb.finish();

        let file = File::open(&file_path).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).unwrap();
            if (*entry.name()).starts_with("Squirrel") {
                let mut buffer = Vec::new();
                entry.read_to_end(&mut buffer).unwrap();

                let cursor = Cursor::new(buffer);
                let mut archive = ZipArchive::new(cursor).unwrap();
                for j in 0..archive.len() {
                    let mut file = archive.by_index(j).unwrap();
                    if !(*file.name()).ends_with(".pkg") {
                        continue;
                    }

                    let temp_dir = tempdir().unwrap();
                    let temp_file = temp_dir.into_path().join(file.name());
                    let mut install_file = File::create(&temp_file).unwrap();
                    copy(&mut file, &mut install_file).unwrap();
                    open(temp_file);
                    break;
                }

                break;
            }
        }
    }

    async fn backup(&self) {
        let max_backups = self.config.max_backups;
        if self.config.max_backups == 0 {
            return;
        }

        let backup_path = self.config.update_dir.join("backups");
        let target = backup_path.join(&self.config.version);
        if target.exists() {
            println!("当前版本已经备份在: {}，略过", target.display());
            return;
        }

        ensure_max_backups(&backup_path, max_backups);

        println!("备份当前版本到：{}", target.display());
        let pb = ProgressBar::new_spinner();
        pb.set_prefix(format!("备份当前版本 {}", &self.config.version));
        pb.set_style(get_spinner_style());
        let source = &self.config.user_dir;
        if let Err(error) = copy_dir_contents(source, &target, |path| {
            pb.set_message(format!("{}", path.display()));
            pb.inc(1);
        }) {
            println!("备份当前版本失败：{error}");
        }
        pb.finish_with_message("完成");
    }

    async fn restore(&self, version: &Path) {
        let from = version;
        let to = &self.config.user_dir;
        fs::remove_dir_all(to).unwrap();

        let pb = ProgressBar::new_spinner();
        pb.set_style(get_spinner_style());
        pb.set_prefix("还原");
        if let Err(error) = copy_dir_contents(from, to, |entry| {
            pb.set_message(format!("{}", entry.display()));
            pb.inc(1);
        }) {
            println!("还原失败：{error}")
        }
        pb.finish_with_message("完成");

        println!("正在重新部署...");
        self.deploy();
    }

    async fn update(&self, release: Release) {
        println!("开始为本地的鼠须管更新声笔输入法...");
        self.backup().await;

        let assets = release.get_assets();
        let m = MultiProgress::new();
        let mut tasks = vec![];

        for asset in assets {
            if !check_file_item(&asset.name, "squirrel", self.config.sentence) {
                continue;
            }

            let name = asset.name;
            let download_url = release.get_download_url(asset.download_url);
            let target_dir = self.config.clone().user_dir;
            let task = tokio::spawn(download_and_install(
                target_dir,
                name,
                download_url,
                m.clone(),
            ));
            tasks.push(task);
        }

        for task in tasks {
            if let Err(error) = task.await {
                println!("更新失败：{error}");
            }
        }

        println!("文件更新完成，重新部署...");
        self.deploy();
    }

    fn deploy(&self) {
        if let Some(exe) = &self.config.exe {
            if let Err(error) = Command::new(exe.as_os_str()).arg("--reload").output() {
                println!("鼠须管重新部署失败: {error}")
            }
        }
    }
}

pub fn get_squirrel() -> Result<Option<Squirrel>, Box<dyn std::error::Error>> {
    if let Ok(exe_path) = grep("[S]quirrel2") {
        let update_dir = work_dir().join("Squirrel");
        let config_file = update_dir.join("config.toml");
        if config_file.exists() {
            // 配置文件存在，直接读取
            let toml = fs::read_to_string(config_file)?;
            let config: IMUpdateConfig = toml::from_str(&toml)?;
            return Ok(Some(Squirrel::new(config)));
        }

        let mut config = IMUpdateConfig {
            name: "Squirrel".to_string(),
            exe: Some(PathBuf::from(exe_path)),
            user_dir: PathBuf::from(std::env::var("HOME").unwrap()).join("Library/Rime"),
            update_dir,
            max_backups: 1,
            sentence: false,
            version: "20051203".to_string(),
        };
        config.write_config();

        println!("Squirrel: {:?}", config);

        return Ok(Some(Squirrel::new(config)));
    }

    Ok(None)
}
