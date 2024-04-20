mod error;
mod fcitx5;
mod hamster;
mod im;
mod release;
mod squirrel;
mod utils;
mod weasel;

use clap::{Arg, ArgAction, Command};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use fcitx5::get_fcitx5;
use hamster::Hamster;
use im::{IMUpdateConfig, InputMethod};
use release::Release;
use squirrel::{get_squirrel, Squirrel};
use std::env::consts::OS;
use std::fs;
use weasel::Weasel;

#[cfg(target_os = "macos")]
async fn install_if_needed(release: &Release) {
    use crate::fcitx5::Fcitx5;

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
                    let selections = ["Squirrel", "Fcitx5", "手动安装", "已安装"];
                    let selected = Select::with_theme(&ColorfulTheme::default())
                        .with_prompt("未在系统中检测到受支持的输入法程序，请选择要安装的输入法")
                        .default(0)
                        .items(&selections)
                        .interact()
                        .unwrap();
                    if selected == 0 {
                        // 将 Squirrel 设置为默认
                        let mut config = Squirrel::default_config();
                        let squirrel = Squirrel::new(config.clone());

                        if let Some(asset) = release
                            .get_assets()
                            .into_iter()
                            .find(|x| x.name.starts_with("squirrel"))
                        {
                            squirrel
                                .install(&asset.name, &release.get_download_url(asset.download_url))
                                .await;
                        }

                        config.write_config();
                    } else if selected == 1 {
                        let mut config = Fcitx5::default_config();
                        config.write_config();
                    } else if selected == 2 {
                        println!("请安装 Squirrel 或 Fcitx5");
                    } else {
                        println!("请先启动 Squirrel 或 Fcitx5");
                    }
                }
                1 => {
                    // 将 squirrel 设置为默认
                    let mut config = squirrel.unwrap().config;
                    config.rename(OS);
                }
                2 => {
                    // 将 fcitx5 设置为默认
                    let mut config = fcitx5.unwrap().config;
                    config.rename(OS);
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

                    let mut config = if selected == 0 {
                        squirrel.unwrap().config
                    } else {
                        fcitx5.unwrap().config
                    };
                    config.rename(OS);

                    println!("已将 {select} 设置为默认，如果要更新 {alter} 请使用 \"sbsrf-udpate {alter}\"", select = selections[selected], alter = selections[1 - selected])
                }
                _ => {
                    // 不可能出现的情况
                }
            }
        }
    }
}

async fn update(
    name: &str,
    force: bool,
    host: Option<&String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(Some(config)) = IMUpdateConfig::new(name) {
        if config.name == "Hamster" && host.is_none() {
            println!("需要用 -H 或 --host 指定远程设备的地址，如：-H 192.168.1.108");
            return Ok(());
        }

        // 获取发布信息
        let release = Release::init().await?;
        let version = release.get_version();

        if version == config.version && !force {
            println!(
                "设备 {name} 上安装的已经是最新版本：{}",
                release.get_version()
            )
        } else {
            if !force {
                println!("{}", style(release.get_release_info()).green());
                println!("新版本 {} 已经发布", style(&version).cyan());
            }

            let info = if force {
                "目标设备上已经是最新版本，是否要覆盖升级？"
            } else {
                "是否要升级到最新版本？"
            };
            let confirmation = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(info)
                .default(true)
                .interact()
                .unwrap();

            if confirmation {
                match config.name.as_str() {
                    "Squirrel" => Squirrel::new(config.clone()).update(release).await,
                    "Hamster" => {
                        let host = host.unwrap();
                        Hamster::new(config.clone(), host.clone())
                            .update(release)
                            .await
                    }
                    "Weasel" => Weasel::new(config.clone()).update(release).await,
                    _ => println!("不支持该输入法下声笔的安装: {name}"),
                }

                let mut new_config = config.clone();
                new_config.save(&version);
            }
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
                "Squirrel" => {
                    Squirrel::new(config.clone())
                        .restore(&backups[selected].path())
                        .await
                }
                "Hamster" => {
                    Hamster::new(config.clone(), host.unwrap().clone())
                        .restore(&backups[selected].path())
                        .await
                }
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

    let m = clap::command!()
        .flatten_help(true)
        .subcommand(
            Command::new("device")
                .about("设备管理")
                .disable_help_flag(true)
                .arg(name_arg.clone())
                .arg(
                    Arg::new("add")
                        .action(ArgAction::SetTrue)
                        .long("add")
                        .short('a')
                        .help("添加设备")
                        .conflicts_with_all(["remove", "edit", "show"]),
                )
                .arg(
                    Arg::new("edit")
                        .action(ArgAction::SetTrue)
                        .long("edit")
                        .short('e')
                        .help("编辑设备信息")
                        .conflicts_with_all(["add", "remove", "show"]),
                )
                .arg(
                    Arg::new("show")
                        .action(ArgAction::SetTrue)
                        .long("show")
                        .short('s')
                        .help("显示设备明细")
                        .conflicts_with_all(["add", "remove", "edit"]),
                )
                .arg(
                    Arg::new("remove")
                        .action(ArgAction::SetTrue)
                        .long("remove")
                        .short('r')
                        .help("移除设备")
                        .conflicts_with_all(["add", "edit", "show"]),
                ),
        )
        .subcommand(
            Command::new("devices")
                .about("设备列表")
                .disable_help_flag(true)
                .arg(
                    Arg::new("more")
                        .short('m')
                        .long("more")
                        .action(ArgAction::SetTrue)
                        .help("显示更多信息"),
                ),
        )
        .subcommand(
            Command::new("update")
                .about("升级词声笔输入法词库")
                .disable_help_flag(true)
                .arg(
                    Arg::new("force")
                        .action(ArgAction::SetTrue)
                        .long("force")
                        .short('f')
                        .help("强制更新"),
                )
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
                .about("清理工作目录")
                .disable_help_flag(true)
                .arg(
                    Arg::new("cache")
                        .long("cache")
                        .short('c')
                        .action(ArgAction::SetTrue),
                ),
        )
        .get_matches();

    match m.subcommand() {
        Some(("device", matches)) => {
            let name = matches.get_one::<String>("name").unwrap();
            if matches.get_flag("add") {
                let mut config = Hamster::default_config(name);
                config.write_config();
                println!("添加完成，配置位于：{}", config.update_dir.display());
            }
            if matches.get_flag("remove") {
                println!("Remove a device: {name}")
            }
            if matches.get_flag("edit") {
                println!("Edit a device: {name}")
            }
            if matches.get_flag("show") {
                println!("Show device info: {name}")
            }
        }
        Some(("devices", matches)) => {
            let more = matches.get_flag("more");
            println!("Print all devices: {more}");
        }
        Some(("update", matches)) => {
            let force = matches.get_flag("force");
            let name = matches.get_one::<String>("name").unwrap();
            let host = matches.try_get_one::<String>("host").unwrap();
            if let Err(error) = update(name, force, host).await {
                eprintln!("更新失败：{}", error)
            }
        }
        Some(("restore", matches)) => {
            let name = matches.get_one::<String>("name").unwrap();
            let host = matches.try_get_one::<String>("host").unwrap();
            restore(name, host).await;
        }

        _ => {
            // 获取发布信息
            let release = Release::init().await?;
            install_if_needed(&release).await;
            update(OS, false, None).await?;
        }
    }

    Ok(())
}

// #[tokio::main]
// async fn main2() -> Result<(), Box<dyn std::error::Error>> {
//     let cli = cli::Cli::parse();
//     let ctx = Context::new(cli.clone());

//     if cli.restore {
//         restore(ctx).await;
//         return Ok(());
//     }

//     let local_version = ctx.config.get_version();
//     let release = Release::init().await?;
//     let release_version = release.get_version();

//     if release_version == local_version && !ctx.force {
//         println!(
//             "{} 上安装的已经是最新版本: {}",
//             style(ctx.platform).cyan(),
//             style(local_version.clone()).cyan()
//         );
//     } else {
//         let force = release_version == local_version && ctx.force;
//         if !force {
//             println!("{}", style(release.get_release_info()).green());
//             println!(
//                 "最新的 Release 版本 {} 已经发布",
//                 style(release.get_version()).cyan()
//             );
//         }

//         let confirmation = Confirm::with_theme(&ColorfulTheme::default())
//             .with_prompt(if force {
//                 "本地已经是最新版本，是否要重新升级？"
//             } else {
//                 "是否要升级到最新版本？"
//             })
//             .default(true)
//             .interact()
//             .unwrap();

//         if confirmation {
//             if ctx.config.get_max_backups() > 0 {
//                 backup(ctx.clone()).await;
//             }

//             if ctx.platform == "ios" {
//                 let confirmation = Confirm::with_theme(&ColorfulTheme::default())
//                     .with_prompt(
//                         "ios 设备是否已经打开 'Wi-Fi 上传方案' 且与当前终端连接到了同一网络？",
//                     )
//                     .default(false)
//                     .interact()
//                     .unwrap();

//                 if !confirmation {
//                     println!("ios 设备升级时需要与当前终端处于同一网络，且已打开仓输入法的 Wi-Fi 上传方案。在更新期间不要关闭 ios 设备屏幕，否则会导致更新失败");
//                     return Ok(());
//                 }
//             }

//             upgrade(release, ctx.clone()).await;

//             let mut config = ctx.config.clone();
//             config.set_version(release_version);
//             config.save();

//             utils::deploy(ctx.rime_home);

//             println!("更新完成");
//         }
//     }

//     Ok(())
// }
