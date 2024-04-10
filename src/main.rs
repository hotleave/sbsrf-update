mod cli;
mod config;
mod release;
mod utils;

use clap::Parser;
use cli::Cli;
use config::Config;
use console::style;
use dialoguer::Confirm;
use indicatif::MultiProgress;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use release::Release;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;
use utils::copy_dir_contents;
use utils::download_file;
use utils::unzip;
use utils::upload_to_ios;

#[derive(Clone)]
struct Context {
    pub working_dir: PathBuf,
    pub force: bool,
    pub platform: String,
    pub remote: bool,
    pub host: String,
    pub config: Config,
}

impl Context {
    pub fn new(cli: Cli) -> Self {
        let platform = cli.platform;
        let working_dir = cli.working_dir.unwrap_or(PathBuf::from(
            Config::path_in_home(".sbsrf-update").join(platform.clone()),
        ));
        let config = config::Config::new(working_dir.clone());
        let remote = platform == "ios";

        Self {
            working_dir,
            platform,
            force: cli.force,
            remote,
            host: cli.host.unwrap_or(String::new()),
            config,
        }
    }
}

fn check_file_item(name: &String, ctx: Context) -> bool {
    if name.starts_with("sbsrf") {
        return true;
    }

    if name.starts_with("octagram") {
        return ctx.config.is_include_octagram();
    }

    return match ctx.platform.as_str() {
        "macos" => name.starts_with("squirrel"),
        "windows" => name.starts_with("weasel"),
        "android" => name.starts_with("trime"),
        _ => false,
    };
}

async fn upgrade(release: Release, ctx: Context) {
    let assets = release.get_assets();
    let mut tasks = vec![];
    let m = MultiProgress::new();

    let cache_dir = ctx.config.working_dir.parent().unwrap().join("cache");
    for _asset in assets {
        let name = _asset.name;
        // let url = format!("http://127.0.0.1:18080{}", _asset.download_url);
        let url = format!("https://gitee.com{}", _asset.download_url);
        let file_path = cache_dir.join(&name);

        if check_file_item(&name, ctx.clone()) {
            let task = tokio::spawn(download_and_install(
                name,
                url.clone(),
                file_path,
                ctx.clone(),
                m.clone(),
            ));
            tasks.push(task);
        }
    }

    // Wait for all downloads to finish
    for task in tasks {
        let _ = task.await.expect("下载或安装失败");
    }
}

async fn download_and_install(
    component: String,
    url: String,
    file_path: PathBuf,
    ctx: Context,
    m: MultiProgress,
) {
    if !file_path.exists() {
        let bar_style = ProgressStyle::with_template("{prefix:.bold} {spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {binary_bytes}/{binary_total_bytes} ({binary_bytes_per_sec}, {eta})").unwrap().progress_chars("#>-");
        let pb = m.add(ProgressBar::new(100));
        pb.set_style(bar_style);
        pb.set_prefix(format!("下载 {}", component));

        if let Err(err) = download_file(url.clone(), file_path.clone(), |len, total| {
            pb.set_length(total);
            pb.inc(len as u64);
        })
        .await
        {
            eprintln!("Error downloading file {}: {}", file_path.display(), err);
            return;
        }
        pb.finish();
    }

    let spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
        .unwrap()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");

    let pb = m.add(ProgressBar::new_spinner());
    let prefix = if ctx.remote {
        format!("解压{}", component)
    } else {
        format!("安装{}", component)
    };
    pb.set_prefix(prefix);
    pb.set_style(spinner_style.clone());

    let output_dir = if ctx.remote {
        tempdir().unwrap().into_path()
    } else {
        ctx.config.get_rime_config_path()
    };
    unzip(file_path, output_dir.clone(), pb).await;

    if ctx.remote {
        let pb = m.add(ProgressBar::new_spinner());
        pb.set_prefix(format!("上传"));
        pb.set_style(spinner_style.clone());
        let _ = upload_to_ios(output_dir.clone(), ctx.host, pb).await;
    }
}

async fn backup(ctx: Context) {
    let source_path = ctx.config.get_rime_config_path();
    if !ctx.remote && !source_path.exists() {
        return;
    }

    let backup_path = ctx.working_dir.join("backup");
    let target_path = backup_path.join(ctx.config.get_version_name());
    if target_path.exists() {
        return;
    }
    fs::create_dir_all(target_path.clone()).unwrap();

    let backups = fs::read_dir(backup_path.clone())
        .unwrap()
        .filter_map(Result::ok);
    let count = backups.count();
    if count >= ctx.config.get_max_backups() as usize {
        let backups = fs::read_dir(backup_path.clone())
            .unwrap()
            .filter_map(Result::ok);
        let mut backup_items: Vec<_> = backups.collect();
        backup_items.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
        let to_be_removed: Vec<_> = backup_items
            .iter()
            .take(count + 1 - ctx.config.get_max_backups() as usize)
            .collect();
        for backup in to_be_removed {
            fs::remove_dir_all(backup.path()).unwrap();
        }
    }

    println!("备份当前版本到：{}", target_path.display());

    if ctx.remote {
        // ios 备份
        let bar_style = ProgressStyle::with_template("{prefix:.bold} {spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {binary_bytes}/{binary_total_bytes} ({binary_bytes_per_sec}, {eta})").unwrap().progress_chars("#>-");
        let pb = ProgressBar::new(100);
        pb.set_style(bar_style);
        pb.set_prefix("备份");

        let url = format!("http://{}/api/raw/Rime", ctx.host);

        if let Err(err) = download_file(url, target_path.join("Rime.zip"), |len, total| {
            pb.set_length(total);
            pb.inc(len as u64);
        })
        .await
        {
            eprintln!("备份失败: {}", err);
        }
        pb.finish();
    } else {
        let spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");
        let pb = ProgressBar::new_spinner();
        pb.set_prefix(format!("备份 {}", ctx.config.get_version_name()));
        pb.set_style(spinner_style);
        copy_dir_contents(&source_path, &target_path, |path| {
            pb.set_message(format!("{}", path.display()));
            pb.inc(1);
        })
        .unwrap();
        pb.finish_with_message("完成");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = cli::Cli::parse();
    let ctx = Context::new(cli);

    let local_version = ctx.config.get_version_id();
    let release = Release::init().await?;
    let release_version = release.get_id();
    let version_name = release.get_version();

    if release_version == local_version && !ctx.force {
        println!(
            "{} 上安装的已经是最新版本: {}",
            style(ctx.platform).cyan(),
            style(version_name.clone()).cyan()
        );
    } else {
        let force = release_version == local_version && ctx.force;
        if !force {
            println!("{}", style(release.get_release_info()).green());
            println!(
                "最新的 Release 版本 {} 已经发布",
                style(release.get_version()).cyan()
            );
        }

        let confirmation = Confirm::new()
            .with_prompt(if force {
                "本地已经是最新版本，是否要重新升级？"
            } else {
                "是否要升级到最新版本？"
            })
            .default(true)
            .interact()
            .unwrap();

        if confirmation {
            if ctx.config.get_max_backups() > 0 {
                backup(ctx.clone()).await;
            }

            if ctx.platform == "ios" {
                let confirmation = Confirm::new()
                    .with_prompt(
                        "ios 设备是否已经打开 'Wi-Fi 上传方案' 且与当前终端连接到了同一网络？",
                    )
                    .default(false)
                    .interact()
                    .unwrap();

                if !confirmation {
                    println!("ios 设备升级时需要与当前终端处于同一网络，且已打开仓输入法的 Wi-Fi 上传方案。在更新期间不要关闭 ios 设备屏幕，否则会导致更新失败");
                    return Ok(());
                }
            }

            upgrade(release, ctx.clone()).await;

            let mut config = ctx.config.clone();
            config.set_version(release_version, version_name);
            config.save();

            println!("更新完成，重新部署即可开始使用新版本");
        }
    }

    Ok(())
}
