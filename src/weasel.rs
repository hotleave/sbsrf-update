use std::{env::consts::OS, fs::{self, File}, io::copy, path::{Path, PathBuf}, process::Command, thread::sleep, time::Duration};

use indicatif::{MultiProgress, ProgressBar};
use tempfile::tempdir;
use zip::ZipArchive;

use crate::{
    im::{check_file_item, IMUpdateConfig, InputMethod},
    utils::{
        copy_dir_contents, download_and_install, download_file, ensure_max_backups, get_bar_style, get_spinner_style, open, work_dir
    },
};

#[derive(Debug)]
pub struct Weasel {
    pub config: IMUpdateConfig,
}

impl Weasel {
    pub fn new(config: IMUpdateConfig) -> Self {
        Self { config }
    }

    pub fn default_config() -> IMUpdateConfig {
        let update_dir = work_dir().join(OS);
        IMUpdateConfig {
            name: "Weasel".to_string(),
            exe: None,
            user_dir: PathBuf::from(std::env::var("APPDATA").unwrap()).join("Rime"),
            update_dir,
            max_backups: 1,
            sentence: false,
            version: "20051203".to_string(),
        }
    }

    fn get_weasel_server_pid() -> i32 {
        let output = Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq WeaselServer.exe"])
            .output()
            .expect("检测小狼毫程序运行状态失败");
        let output_str = String::from_utf8_lossy(&output.stdout);

        // 映像名称                       PID 会话名              会话#       内存使用
        // ========================= ======== ================ =========== ============
        // WeaselServer.exe              7528 Console                    1     10,068 K
        if output_str.contains("WeaselServer.exe") {
            let mut splited = output_str.split("\r\n");
            if let Some(line) = splited.nth(3) {
                let mut splited = line.split_ascii_whitespace();

                let pid = splited.nth(1).unwrap_or("-1");
                return pid.parse::<i32>().unwrap();
            }
        }

        -1
    }

    fn get_weasel_exe(pid: i32) -> Option<PathBuf> {
        let arg = format!("ProcessId={pid}");
        let output = Command::new("wmic")
            .args(&["process", "where", &arg, "get", "ExecutablePath"])
            .output().ok()?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut splited = output_str.split("\r\n");
        if let Some(path) = splited.nth(1) {
            Some(PathBuf::from(path.trim()))
        } else {
            None
        }
    }

    fn toggle_weasel_server_state(&self, start: bool) {
        if let Some(exe_path) = self.config.clone().exe {
            let mut cmd = Command::new(exe_path.as_os_str());
            if !start {
                cmd.arg("/q");
            }
            cmd.spawn().unwrap();
        }
    }

    pub fn running(&self) -> bool {
        Weasel::get_weasel_server_pid() > 0
    }

    pub fn start(&self) {
        self.toggle_weasel_server_state(true)
    }

    pub fn stop(&self) {
        self.toggle_weasel_server_state(false)
    }
}

impl InputMethod for Weasel {
    async fn install(&self, name: &str, download_url: &str) {
        println!("准备安装小狼毫程序");
        let file_path = work_dir().join("_cache").join(name);
        if !file_path.exists() {
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
        }

        let file = File::open(&file_path).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        let temp_dir = tempdir().unwrap().into_path();
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).unwrap();
            if (*entry.name()).ends_with(".exe") {
                let temp_file = temp_dir.join(entry.name());
                let mut install_file = File::create(&temp_file).unwrap();
                copy(&mut entry, &mut install_file).unwrap();
                drop(install_file);
                open(temp_file);
                break;
            }
        }

        while !self.running() {
            sleep(Duration::from_secs(1));
        }
        if let Some(exe) = Weasel::get_weasel_exe(Weasel::get_weasel_server_pid()) {
            let mut config = self.config.clone();
            config.exe = Some(exe);
            config.write_config();

            println!("Weasel 安装完成");
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
        let running = self.running();
        if running {
            println!("检测到小狼毫程序正在运行，暂时停止");
            self.stop();
            while self.running() {
                sleep(Duration::from_secs(1));
            }
        }

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

        if running {
            self.start();
            while !self.running() {
                sleep(Duration::from_secs(1));
            }
            println!("小狼毫程序已恢复启动");
        }

        println!("正在重新部署...");
        self.deploy();
    }

    async fn update(&self, release: crate::release::Release) {
        let running = self.running();
        if running {
            println!("检测到小狼毫程序正在运行，暂时停止");
            self.stop();
            while self.running() {
                sleep(Duration::from_secs(1));
            }
        }

        println!("开始为本地的小狼毫更新声笔输入法...");
        self.backup().await;

        let assets = release.get_assets();
        let m = MultiProgress::new();
        let mut tasks = vec![];

        for asset in assets {
            if !check_file_item(&asset.name, "weasel", self.config.sentence) {
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

        if running {
            self.start();
            while !self.running() {
                sleep(Duration::from_secs(1));
            }
            println!("小狼毫程序已恢复启动");
        }

        println!("文件更新完成，重新部署...");
        self.deploy();
    }

    fn deploy(&self) {
        if let Some(exe_path) = self.config.clone().exe {
            let home = exe_path.parent().unwrap();
            let mut cmd = Command::new(home.join("WeaselDeployer.exe").as_os_str());
            cmd.spawn().unwrap();
        }
    }
}

pub fn get_weasel() -> Result<Option<Weasel>, Box<dyn std::error::Error>>{
    let pid = Weasel::get_weasel_server_pid();
    if pid > 0 {
        let update_dir = work_dir().join(OS);
        let config_file = update_dir.join("config.toml");
        if config_file.exists() {
            // 配置文件存在，直接读取
            let toml = fs::read_to_string(config_file)?;
            let config: IMUpdateConfig = toml::from_str(&toml)?;
            return Ok(Some(Weasel::new(config)));
        }

        if let Some(exe) = Weasel::get_weasel_exe(pid) {
            let mut config = Weasel::default_config();
            config.exe = Some(exe);
            config.write_config();

            return Ok(Some(Weasel::new(config)));
        }
    }

    Ok(None)
}