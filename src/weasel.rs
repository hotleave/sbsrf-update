use std::{path::PathBuf, process::Command};

use crate::im::{IMUpdateConfig, InputMethod};


#[derive(Debug)]
pub struct Weasel {
    pub config: IMUpdateConfig,
}

impl Weasel {
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
                return pid.parse::<i32>().unwrap()
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
}


impl InputMethod for Weasel {
    fn running(&self) -> bool {
        Weasel::get_weasel_server_pid() > 0
    }

    fn start(&self) {
        self.toggle_weasel_server_state(true)
    }

    fn stop(&self) {
        self.toggle_weasel_server_state(false)
    }

    async fn install(&self, name: &str, download_url: &str) {
        todo!()
    }

    fn backup(&self) {
        todo!()
    }

    fn restore(&self, version: &PathBuf) {
        todo!()
    }

    async fn update(&self, release: crate::release::Release) {
        
    }

    fn deploy(&self) {
        if let Some(exe_path) = self.config.clone().exe {
            let home = exe_path.parent().unwrap();
            let mut cmd = Command::new(home.join("WeaselDeployer.exe").as_os_str());
            cmd.spawn().unwrap();
        }
    }
}