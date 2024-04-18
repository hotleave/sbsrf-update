use std::{env::consts::OS, fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{error::Error, release::AttachFile};

#[derive(Deserialize, Serialize, Clone)]
pub struct Device {
    /// 主目录
    pub base_dir: PathBuf,
    /// 最大备份数量
    pub max_backups: i32,
    /// 是否使用整句世入方案
    pub sentence: bool,
    /// 设备操作系统
    pub platform: String,
    /// 当前版本
    pub version: String,
}

impl Device {
    pub fn new(work_dir: PathBuf, name: &str) -> Result<Device, Box<dyn std::error::Error>> {
        let config_file = work_dir.join(name).join("config.toml");
        if config_file.exists() {
            // 配置文件存在，直接读取
            let toml = std::fs::read_to_string(config_file)?;
            let device: Device = toml::from_str(&toml)?;
            return Ok(device);
        }

        if name == OS {
            // 本地设备，生成默认配置
            let device = Self {
                base_dir: work_dir.join(name),
                max_backups: 1,
                sentence: false,
                platform: OS.to_string(),
                version: String::from("20051203"),
            };

            return Ok(device);
        }

        Err(Box::new(Error::new(&format!("设备{name}的配置文件不存在"))))
    }

    pub fn save(&mut self, version: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.version = version.to_string();

        if !self.base_dir.exists() {
            fs::create_dir_all(self.base_dir.clone())?;
        }

        let content = toml::to_string(self).unwrap();
        fs::write(self.base_dir.join("config.toml"), content)?;

        Ok(())
    }
}

pub trait TargetDevice {
    /**
     * 安装
     */
    fn install();

    /**
     * 备份
     */
    fn backup();

    /**
     * 更新
     */
    fn update(assets: Vec<AttachFile>);

    /**
     * 回滚
     */
    fn restore();
}
