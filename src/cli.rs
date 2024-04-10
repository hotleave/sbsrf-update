use std::path::PathBuf;

use clap::Parser;

/// 声笔输入法更新程序
#[derive(Parser, Debug, Clone)]
#[command(version, about)]
pub struct Cli {
    /// 工作目录，默认在 $HOME/.sbsrf-update
    #[arg(short, long)]
    pub working_dir: Option<PathBuf>,

    /// 强制更新，默认本地版本和服务器版本一致时不作任何操作，强制更新时即使版本相同也会更新
    #[arg(short, long)]
    pub force: bool,

    /// 目标操作系统，默认为当前系统
    #[arg(short, long, default_value_t = String::from(std::env::consts::OS))]
    pub platform: String,

    /// iOS 设备 IP 地址
    #[arg(long)]
    pub host: Option<String>,

    /// 还原备份的版本
    #[arg(long)]
    pub restore: bool,
}
