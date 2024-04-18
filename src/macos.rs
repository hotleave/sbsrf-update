use core::str;
use std::{
    env::consts::OS, fs, path::PathBuf, process::{Command, Stdio}
};

use indicatif::{MultiProgress, ProgressBar};

use crate::{
    im::{check_file_item, IMUpdateConfig, InputMethod},
    release::Release,
    utils::{copy_dir_contents, download_file, ensure_max_backups, get_bar_style, get_spinner_style, unzip, work_dir},
};

fn grep(keyword: &str) -> Result<String, Box<dyn std::error::Error>> {
    let ps = Command::new("ps")
        .arg("aux")
        .stdout(Stdio::piped())
        .spawn()
        .expect("ps 命令失败");

    let grep = Command::new("grep")
        .arg(keyword)
        .stdin(ps.stdout.unwrap())
        .stdout(Stdio::piped())
        .spawn()
        .expect("grep 命令失败");

    let tr = Command::new("tr")
        .args(["-s", " "])
        .stdin(grep.stdout.unwrap())
        .stdout(Stdio::piped())
        .spawn()
        .expect("tr 命令失败");

    let output = Command::new("cut")
        .args(["-d", " ", "-f", "11-"])
        .stdin(tr.stdout.unwrap())
        .output()
        .expect("查找 Squirrel 进程失败");

    let output_str = String::from_utf8(output.stdout).unwrap();
    Ok(output_str.trim().to_string())
}

#[derive(Debug)]
pub struct Squirrel {
    pub config: IMUpdateConfig,
}

impl Squirrel {
    pub fn new(config: IMUpdateConfig) -> Self {
        Self { config }
    }
}

async fn download_and_install(config: IMUpdateConfig, name: String, url: String, m: MultiProgress) {
    let cache_dir = work_dir().join("_cache");
    let file_path = cache_dir.join(&name);

    if !file_path.exists() {
        // 下载文件
        let pb = m.add(ProgressBar::new(100));
        pb.set_prefix(format!("下载 {}", &name));
        pb.set_style(get_bar_style());

        if let Err(error) = download_file(url.to_string(), &file_path, |len, total| {
            pb.set_length(total);
            pb.inc(len as u64);
        })
        .await
        {
            println!("下载文件{}失败: {error}", &name);
        }

        pb.finish();
    }

    // 解压
    let pb = m.add(ProgressBar::new_spinner());
    pb.set_prefix(format!("更新 {}", &name));
    pb.set_style(get_spinner_style());
    unzip(&file_path, &config.user_dir, pb).await;
}

impl InputMethod for Squirrel {
    fn running(&self) -> bool {
        todo!()
    }

    fn start(&self) {
        todo!()
    }

    fn stop(&self) {
        todo!()
    }

    fn install(&self) {
        todo!()
    }

    fn backup(&self) {
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

    fn restore(&self) {
        todo!()
    }

    async fn update(&self, release: Release) {
        println!("开始为本地的鼠须管更新声笔输入法...");
        self.backup();

        let assets = release.get_assets();
        let m = MultiProgress::new();
        let mut tasks = vec![];

        for asset in assets {
            if !check_file_item(&asset.name, "squirrel", self.config.sentence) {
                continue;
            }

            let name = asset.name;
            let download_url = release.get_download_url(asset.download_url);
            let task = tokio::spawn(download_and_install(
                self.config.clone(),
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
    if let Ok(exe_path) = grep("[S]quirrel") {
        let update_dir = work_dir().join(OS);
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
        config.save(&config.version.clone());

        return Ok(Some(Squirrel::new(config)));
    }

    Ok(None)
}

impl Fcitx5 {
    pub fn new(config: IMUpdateConfig) -> Self {
        Self { config }
    }
}

#[derive(Debug)]
pub struct Fcitx5 {
    pub config: IMUpdateConfig,
}

pub fn get_fcitx5() -> Result<Option<Fcitx5>, Box<dyn std::error::Error>> {
    if let Ok(exe_path) = grep("[F]citx5") {
        let update_dir = work_dir().join("Fcitx5");
        let config_file = update_dir.join("config.toml");
        if config_file.exists() {
            // 配置文件存在，直接读取
            let toml = fs::read_to_string(config_file)?;
            let config: IMUpdateConfig = toml::from_str(&toml)?;
            return Ok(Some(Fcitx5::new(config)));
        }

        let mut config = IMUpdateConfig {
            // id: "Fcitx5".to_string(),
            name: "Fcitx5".to_string(),
            exe: Some(PathBuf::from(exe_path)),
            user_dir: PathBuf::from(std::env::var("HOME").unwrap())
                .join(".local/share/fcitx5/rime"),
            update_dir,
            max_backups: 1,
            sentence: false,
            version: "20051203".to_string(),
        };
        config.save(&config.version.clone());
        return Ok(Some(Fcitx5::new(config)));
    }

    Ok(None)
}
