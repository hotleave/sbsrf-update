use std::{fs, path::PathBuf};
use serde::{Deserialize, Serialize};

use crate::{release::Release, utils::work_dir};

pub fn check_file_item(name: &str, im: &str, sentence: bool) -> bool {
    if name.starts_with("sbsrf") {
        return true;
    }

    if name.starts_with("octagram") {
        return sentence;
    }

    name.starts_with(&im.to_lowercase())
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct IMUpdateConfig {
    /// 输入法名称
    pub name: String,
    /// 可执行文件
    pub exe: Option<PathBuf>,
    /// Rime 用户目录
    pub user_dir: PathBuf,
    /// 更新目录
    pub update_dir: PathBuf,
    /// 最大备份数量
    pub max_backups: i32,
    /// 是否使用整句世入方案
    pub sentence: bool,
    /// 当前版本
    pub version: String,
}

impl IMUpdateConfig {
    pub fn new(name: &str) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let config_file = work_dir().join(name).join("config.toml");
        if config_file.exists() {
            // 配置文件存在，直接读取
            let toml = std::fs::read_to_string(config_file)?;
            let config: Self = toml::from_str(&toml)?;
            return Ok(Some(config));
        }

        Ok(None)
    }

    pub fn save(&mut self, version: &str) {
        self.version = version.to_string();
        let content = toml::to_string(self).unwrap();
        fs::write(self.update_dir.join("config.toml"), content).unwrap();
    }
}

pub trait InputMethod {
    /**
     * 是否在运行
     */
    fn running(&self) -> bool;

    /**
     * 启动
     */
    fn start(&self);

    /**
     * 停止
     */
    fn stop(&self);
    /**
     * 安装
     */
    fn install(&self);

    /**
     * 备份
     */
    fn backup(&self);

    /**
     * 回滚
     */
    fn restore(&self);

    /**
     * 更新
     */
    async fn update(&self, release: Release);

    /**
     * 部署
     */
    fn deploy(&self);
}
