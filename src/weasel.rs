use std::{env::consts::OS, fs, path::{Path, PathBuf}, process::Command, thread::sleep, time::Duration};

use indicatif::{MultiProgress, ProgressBar};

use crate::{
    im::{check_file_item, IMUpdateConfig, InputMethod},
    utils::{
        copy_dir_contents, download_and_install, ensure_max_backups, get_spinner_style, work_dir,
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
        todo!()
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

        let running = self.running();
        if running {
            println!("检测到小狼毫程序正在运行，暂时停止");
            self.stop();
            sleep(Duration::from_secs(1));
        }

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

        if running {
            self.start();
            println!("小狼毫程序已恢复启动");
        }

        pb.finish_with_message("完成");
    }

    async fn restore(&self, version: &Path) {
        let running = self.running();
        if running {
            println!("检测到小狼毫程序正在运行，暂时停止");
            self.stop();
            sleep(Duration::from_secs(1));
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
            println!("小狼毫程序已恢复启动");
        }

        println!("正在重新部署...");
        self.deploy();
    }

    async fn update(&self, release: crate::release::Release) {
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
