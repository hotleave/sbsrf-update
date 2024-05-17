use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use indicatif::{MultiProgress, ProgressBar};

use crate::{
    im::{check_file_item, IMUpdateConfig, InputMethod},
    release::Release,
    utils::{
        copy_dir_contents, download_and_install, download_file, ensure_max_backups, get_bar_style,
        get_spinner_style, grep, open, work_dir,
    },
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
        let update_dir = work_dir().join("Fcitx5");
        IMUpdateConfig {
            name: "Fcitx5".to_string(),
            exe: Some(PathBuf::from(
                "/Library/Input Methods/Fcitx5.app/Contents/MacOS/Fcitx5",
            )),
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
    async fn install(&self, _: &str, _: &str) {
        let zip_file_path = work_dir().join("_cache/Fcitx5-Rime.zip");
        if !zip_file_path.exists() {
            let url = "https://github.com/fcitx-contrib/fcitx5-macos-installer/releases/download/latest/Fcitx5-Rime.zip";
            let pb = ProgressBar::new(100);
            pb.set_prefix("下载 Fcitx5-Rime.zip");
            pb.set_style(get_bar_style());
            if let Err(error) = download_file(url.to_string(), &zip_file_path, |len, total| {
                pb.set_length(total);
                pb.inc(len as u64);
            })
            .await
            {
                println!("下载文件 Fcitx5-Rime.zip 失败: {error}");
            }
            pb.finish();
        }

        let app_path = work_dir().join("_cache/Fcitx5Installer.app");
        if app_path.exists() {
            fs::remove_dir_all(app_path.clone()).unwrap();
        }

        if let Err(error) = Command::new("unzip")
            .args([
                "-q",
                zip_file_path.to_str().unwrap(),
                "-d",
                work_dir().join("_cache").to_str().unwrap(),
            ])
            .status()
        {
            println!("解压文件失败：{error}");
            return;
        }
        open(app_path);
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
        println!("开始为本地的小企鹅更新声笔输入法...");
        self.backup().await;

        let m = MultiProgress::new();
        let mut tasks = vec![];

        for asset in release.assets {
            if !check_file_item(&asset.name, "fcitx5", self.config.sentence) {
                continue;
            }

            let name = asset.name;
            let download_url = asset.download_url;
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
        if let Some(exe) = self.config.clone().exe {
            let mut ancestors = exe.ancestors();
            if let Some(contents) = ancestors.nth(2) {
                let fcitx5_curl = contents.to_path_buf().join("bin/fcitx5-curl");
                Command::new(fcitx5_curl)
                    .args(["/config/addon/rime/deploy", "-X", "POST", "-d", "{}"])
                    .spawn()
                    .expect("部署失败");
            }
        }
    }
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

        let mut config = Fcitx5::default_config();
        config.exe = Some(PathBuf::from(&exe_path));
        config.save(&config.version.clone());

        return Ok(Some(Fcitx5::new(config)));
    }

    Ok(None)
}
