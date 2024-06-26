#[cfg(target_os = "macos")]
mod fcitx5;
mod hamster;
mod im;
mod release;
#[cfg(target_os = "macos")]
mod squirrel;
mod utils;
#[cfg(target_os = "windows")]
mod weasel;

use clap::{Arg, ArgAction, Command};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use hamster::Hamster;
use im::{IMUpdateConfig, InputMethod};
use release::Release;
use std::io::Write;
use std::{env::consts::OS, fs::read_to_string};
use std::fs::{self, create_dir_all};
use utils::{open, work_dir};

#[cfg(target_os = "macos")]
use {
    fcitx5::{get_fcitx5, Fcitx5},
    squirrel::{get_squirrel, Squirrel},
};

#[cfg(target_os = "windows")]
use weasel::{get_weasel, Weasel};

#[cfg(target_os = "macos")]
async fn install_if_needed(release: &Release) {
    if let Ok(Some(_)) = IMUpdateConfig::new(OS) {
        return;
    }

    if let Ok(squirrel) = get_squirrel() {
        if let Ok(fcitx5) = get_fcitx5() {
            let mut result = 0;
            result |= if squirrel.is_none() { 0 } else { 1 };
            result |= if fcitx5.is_none() { 0 } else { 2 };
            match result {
                0 => {
                    // 由用户选择需要安装的输入法
                    let selections = ["安装鼠须管程序", "安装小企鹅程序", "手动下载安装", "已安装但未启动"];
                    let selected = Select::with_theme(&ColorfulTheme::default())
                        .with_prompt("未在系统中检测到受支持的输入法程序")
                        .default(0)
                        .items(&selections)
                        .interact()
                        .unwrap();
                    if selected == 0 {
                        // 将 Squirrel 设置为默认
                        let mut config = Squirrel::default_config();
                        let squirrel = Squirrel::new(config.clone());

                        if let Some(asset) = release
                            .assets
                            .iter()
                            .find(|x| x.name.starts_with("squirrel"))
                        {
                            squirrel.install(&asset.name, &asset.download_url).await;
                        }

                        config.write_config();
                        config.make_default();
                    } else if selected == 1 {
                        let mut config = Fcitx5::default_config();
                        let fcitx5 = Fcitx5::new(config.clone());
                        fcitx5.install("", "").await;
                        config.write_config();
                        config.make_default();
                    } else if selected == 2 {
                        println!("请安装 鼠须管 或 小企鹅 程序");
                    } else {
                        println!("请先启动 鼠须管 或 小企鹅 程序");
                    }
                }
                1 => {
                    // 将 squirrel 设置为默认
                    let config = squirrel.unwrap().config;
                    config.make_default();
                    // config.rename(OS);
                }
                2 => {
                    // 将 fcitx5 设置为默认
                    let config = fcitx5.unwrap().config;
                    config.make_default();
                }
                3 => {
                    // 由用户选择默认
                    let selections = ["Squirrel", "Fcitx5"];
                    let selected = Select::with_theme(&ColorfulTheme::default())
                        .with_prompt("发现多个受支持的输入法，请选择默认更新的输入法")
                        .default(0)
                        .items(&selections)
                        .interact()
                        .unwrap();

                    let config = if selected == 0 {
                        squirrel.unwrap().config
                    } else {
                        fcitx5.unwrap().config
                    };
                    config.make_default();

                    println!("已将 {select} 设置为默认，如果要更新 {alter} 请使用 \"sbsrf-udpate {alter}\"", select = selections[selected], alter = selections[1 - selected])
                }
                _ => {
                    // 不可能出现的情况
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
async fn install_if_needed(release: &Release) {
    if let Ok(Some(_)) = IMUpdateConfig::new(OS) {
        return;
    }

    if let Ok(weasel) = get_weasel() {
        if weasel.is_none() {
            let selections = ["安装小狼毫", "手动下载安装", "已安装但未启动"];
            let selected = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("未在系统中检测到受支持的输入法程序")
                .default(0)
                .items(&selections)
                .interact()
                .unwrap();
            
            match selected {
                0 => {
                    let config = Weasel::default_config();
                    let weasel = Weasel::new(config);

                    if let Some(asset) = release.assets.iter().find(|x| x.name.starts_with("weasel")) {
                        weasel.install(&asset.name, &asset.download_url).await;
                    }
                },

                1 => {
                    println!("请先启动 鼠须管 或 小企鹅 程序");
                },

                _ => {
                    println!("请安装 小狼毫 程序");
                }
            }
        }
    }
}

async fn update(
    release: Release,
    name: &str,
    host: Option<&String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(Some(config)) = IMUpdateConfig::new(name) {
        if config.name == "Hamster" && host.is_none() {
            println!("需要用 -H 或 --host 指定远程设备的地址，如：-H 192.168.1.108");
            return Ok(());
        }

        // 获取发布信息
        let version = release.clone().version;
        let force = version == config.version;

        let prompt = if force {
            "目标设备上安装的已经是最新版本，是否要覆盖升级？"
        } else {
            println!("{}", style(release.clone().intro).green());
            println!("新版本 {} 已经发布", style(&version).cyan());

            "是否要升级到最新版本？"
        };

        let confirmation = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .default(!force)
            .interact()
            .unwrap();

        if confirmation {
            let cache_dir = work_dir().join("_cache");

            // 检测缓存目录中的文件版本
            let info_path = cache_dir.join("version.info");
            let cache_version = if info_path.exists() { read_to_string(&info_path)? } else { "0".to_string() };
            if cache_version != version {
                if cache_dir.exists() {
                    println!("清理缓存目录...");
                    fs::remove_dir_all(&cache_dir).unwrap();
                }

                create_dir_all(&cache_dir)?;
                let mut info_file = fs::File::create(&info_path)?;
                info_file.write_all(version.as_bytes())?;
            }

            match config.name.as_str() {
                #[cfg(target_os = "macos")]
                "Squirrel" => Squirrel::new(config.clone()).update(release.clone()).await,

                #[cfg(target_os = "macos")]
                "Fcitx5" => Fcitx5::new(config.clone()).update(release.clone()).await,

                "Hamster" => {
                    let host = host.unwrap();
                    Hamster::new(config.clone(), host.clone())
                        .update(release.clone())
                        .await
                }
                #[cfg(target_os = "windows")]
                "Weasel" => Weasel::new(config.clone()).update(release.clone()).await,
                _ => println!("不支持该输入法下声笔的安装: {name}"),
            }

            let mut new_config = config.clone();
            new_config.save(&version);
        }
    } else {
        println!("指定的设备不存在：{name}");
    }

    Ok(())
}

async fn restore(name: &str, host: Option<&String>) {
    if let Ok(Some(config)) = IMUpdateConfig::new(name) {
        if config.name == "Hamster" && host.is_none() {
            println!("需要用 -H 或 --host 指定远程设备的地址，如：-H 192.168.1.108");
            return;
        }

        let backup_path = config.update_dir.join("backups");
        println!("backup_path={:?}", backup_path);
        let mut backups: Vec<fs::DirEntry> = fs::read_dir(&backup_path)
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        backups.sort_by_key(|x| x.file_name());

        let selections: Vec<String> = backups
            .iter()
            .map(|e| {
                return e.file_name().to_str().unwrap().to_string();
            })
            .collect();
        let selected = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("选择要恢复的版本")
            .default(selections.len() - 1)
            .items(&selections)
            .interact()
            .unwrap();

        let confirmation = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("确认要恢复到 {} 版本吗？", selections[selected]))
            .default(false)
            .interact()
            .unwrap();

        if confirmation {
            match config.name.as_str() {
                #[cfg(target_os = "macos")]
                "Squirrel" => {
                    Squirrel::new(config.clone())
                        .restore(&backups[selected].path())
                        .await
                }

                #[cfg(target_os = "macos")]
                "Fcitx5" => {
                    Fcitx5::new(config.clone())
                        .restore(&backups[selected].path())
                        .await
                }
                "Hamster" => {
                    Hamster::new(config.clone(), host.unwrap().clone())
                        .restore(&backups[selected].path())
                        .await
                }

                #[cfg(target_os = "windows")]
                "Weasel" => {
                    Weasel::new(config.clone())
                        .restore(&backups[selected].path())
                        .await
                }
                _ => println!("不支持该输入法下声笔的还原操作: {name}"),
            }

            let mut new_config = config.clone();
            new_config.save(&selections[selected]);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let name_arg = Arg::new("name").default_value(OS).help("设备唯一名称");
    let host_arg = Arg::new("host")
        .long("host")
        .short('H')
        .help("远程设备地址");

    let mut device_command = Command::new("device")
        .about("设备管理")
        .subcommand(
            Command::new("list")
                .about("显示设备列表")
                .disable_help_flag(true),
        )
        .subcommand(
            Command::new("add")
                .about("添加远程设备")
                .disable_help_flag(true)
                .arg(name_arg.clone()),
        )
        .subcommand(
            Command::new("edit")
                .about("编辑设备信息")
                .disable_help_flag(true)
                .arg(name_arg.clone()),
        )
        .subcommand(
            Command::new("show")
                .about("显示设备明细")
                .disable_help_flag(true)
                .arg(name_arg.clone()),
        )
        .subcommand(
            Command::new("remove")
                .about("移除远程设备")
                .disable_help_flag(true)
                .arg(name_arg.clone()),
        );

    if OS == "macos" {
        device_command = device_command.clone().subcommand(
            Command::new("default")
                .about("设置默认设备")
                .disable_help_flag(true)
                .arg(name_arg.clone()),
        );
    }

    let m = clap::command!()
        .flatten_help(true)
        .subcommand(&device_command)
        .subcommand(
            Command::new("update")
                .about("升级词声笔输入法词库")
                .disable_help_flag(true)
                .arg(host_arg.clone())
                .arg(name_arg.clone()),
        )
        .subcommand(
            Command::new("restore")
                .about("还原到某个备份版本")
                .disable_help_flag(true)
                .arg(host_arg.clone())
                .arg(name_arg.clone()),
        )
        .subcommand(
            Command::new("clean")
                .about("清理工作目录缓存")
                .disable_help_flag(true)
                .arg(
                    Arg::new("all")
                        .long("all")
                        .short('a')
                        .help("删除整个工作目录，包含设备及备份")
                        .action(ArgAction::SetTrue),
                ),
        )
        .get_matches();

    match m.subcommand() {
        Some(("device", matches)) => match matches.subcommand() {
            Some(("list", _)) => {
                if let Ok(entries) = fs::read_dir(work_dir()) {
                    let mut entries = entries;
                    while let Some(Ok(entry)) = entries.next() {
                        let name = entry.file_name().into_string().unwrap();
                        if name.starts_with('_') {
                            continue;
                        }

                        let tic = if name == OS { "->" } else { "  " };
                        println!("{} {}", tic, entry.file_name().to_str().unwrap());
                    }
                }
            }
            Some(("add", add_matches)) => {
                let name = add_matches.get_one::<String>("name").unwrap();
                let mut config = Hamster::default_config(name);
                config.write_config();
                println!("添加完成，配置位于：{}", config.update_dir.display());
            }
            Some(("remove", remove_matches)) => {
                let name = remove_matches.get_one::<String>("name").unwrap();
                let confirmation = Confirm::new()
                    .with_prompt("备份内容已将被删除，且不可恢复, 确认要删除整个工作目录吗？")
                    .default(false)
                    .interact()
                    .unwrap();

                if confirmation {
                    let dir = work_dir().join(name);
                    if dir.exists() {
                        fs::remove_dir_all(dir).unwrap();
                    }

                    println!("设备 {name} 的配置已移除");
                }
            }
            Some(("edit", edit_matches)) => {
                let name = edit_matches.get_one::<String>("name").unwrap();
                let config_path = work_dir().join(name).join("config.toml");
                open(config_path);
            }
            Some(("show", show_matches)) => {
                let name = show_matches.get_one::<String>("name").unwrap();
                let config_path = work_dir().join(name).join("config.toml");
                if config_path.exists() {
                    match fs::read_to_string(config_path) {
                        Ok(content) => println!("{content}"),
                        Err(error) => println!("未找到设备 {name} 的配置信息: {error}"),
                    }
                }
            }

            #[cfg(target_os = "macos")]
            Some(("default", default_matches)) => {
                let name = default_matches.get_one::<String>("name").unwrap();
                if let Ok(Some(config)) = IMUpdateConfig::new(name) {
                    config.make_default();

                    println!("已将 {name} 设置为默认设备");
                }
            }

            _ => {
                println!("不支持的命令");
            }
        },
        Some(("update", matches)) => {
            let name = matches.get_one::<String>("name").unwrap();
            let host = matches.try_get_one::<String>("host").unwrap();
            let release = Release::init().await?;
            if let Err(error) = update(release, name, host).await {
                eprintln!("更新失败：{}", error)
            }
        }
        Some(("restore", matches)) => {
            let name = matches.get_one::<String>("name").unwrap();
            let host = matches.try_get_one::<String>("host").unwrap();
            restore(name, host).await;
        }

        Some(("clean", matches)) => {
            let all = matches.get_flag("all");
            if all {
                let confirmation = Confirm::new()
                    .with_prompt("备份内容已将被删除，且不可恢复, 确认要删除整个工作目录吗？")
                    .default(false)
                    .interact()
                    .unwrap();
                if confirmation {
                    fs::remove_dir_all(work_dir()).unwrap();
                }
            } else {
                fs::remove_dir_all(work_dir().join("_cache")).unwrap();
                println!("缓存目录已被清理");
            }
        }
        _ => {
            // 获取发布信息
            let release = Release::init().await?;
            install_if_needed(&release).await;
            update(release, OS, None).await?;
        }
    }

    Ok(())
}
