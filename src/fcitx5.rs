use std::{
    env::consts::OS, fs, path::PathBuf, process::{Command, Stdio}
};

use indicatif::{MultiProgress, ProgressBar};

use crate::{
    im::{check_file_item, IMUpdateConfig, InputMethod},
    release::Release,
    utils::{copy_dir_contents, download_file, ensure_max_backups, get_bar_style, get_spinner_style, grep, unzip, work_dir},
};

#[derive(Debug)]
pub struct Fcitx5 {
    pub config: IMUpdateConfig,
}

impl Fcitx5 {
    pub fn new(config: IMUpdateConfig) -> Self {
        Self { config }
    }

    pub fn default_config() -> IMUpdateConfig {
        let update_dir = work_dir().join(OS);
        IMUpdateConfig {
            name: "Fcitx5".to_string(),
            exe: Some(PathBuf::from("/Library/Input Methods/Fcitx5.app/Contents/MacOS/Fcitx5")),
            user_dir: PathBuf::from(std::env::var("HOME").unwrap())
                .join(".local/share/fcitx5/rime"),
            update_dir,
            max_backups: 1,
            sentence: false,
            version: "20051203".to_string(),
        }
    }
}

impl InputMethod for Fcitx5 {
    fn running(&self) -> bool {
        todo!()
    }

    fn start(&self) {
        todo!()
    }

    fn stop(&self) {
        todo!()
    }

    async fn install(&self, name: &str, download_url: &str) {
        todo!()
    }

    async fn backup(&self) {
        todo!()
    }

    async fn restore(&self, version: &PathBuf) {
        todo!()
    }

    async fn update(&self, release: Release) {
        todo!()
    }

    fn deploy(&self) {
        todo!()
    }
}

pub fn get_fcitx5() -> Result<Option<Fcitx5>, Box<dyn std::error::Error>> {
    if let Ok(exe_path) = grep("[F]citx52") {
        let update_dir = work_dir().join("Fcitx5");
        let config_file = update_dir.join("config.toml");
        if config_file.exists() {
            // 配置文件存在，直接读取
            let toml = fs::read_to_string(config_file)?;
            let config: IMUpdateConfig = toml::from_str(&toml)?;
            return Ok(Some(Fcitx5::new(config)));
        }

        let mut config = IMUpdateConfig {
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
        println!("Squirrel: {:?}", config);

        return Ok(Some(Fcitx5::new(config)));
    }

    Ok(None)
}