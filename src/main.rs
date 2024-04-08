mod config;
mod utils;
mod release;
mod cli;

use std::fs;
use std::path::PathBuf;
use config::Config;
use indicatif::MultiProgress;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use release::Release;
use utils::unzip;
use utils::download_file;
use utils::copy_dir_contents;
use clap::Parser;
use console::style;
use dialoguer::Confirm;

fn check_file_item(name: &String, os: &str, config: Config) -> bool {
    if name.starts_with("sbsrf") {
        return true;
    }

    if name.starts_with("octagram") {
        return config.is_include_octagram();
    }

    return match os {
        "macos" => name.starts_with("squirrel"),
        "windows" => name.starts_with("weasel"),
        "android" => name.starts_with("trime"),
        _ => false,
    }
}

async fn upgrade(release: Release, config: Config, os: &str) {
    let assets = release.get_assets();
    let mut tasks = vec![];
    let m = MultiProgress::new();

    for _asset in assets {
        let name = _asset.name;
        // let url = format!("http://127.0.0.1:18080{}", _asset.download_url);
        let url = format!("https://gitee.com{}", _asset.download_url);
        let file_path = config.working_dir.parent().unwrap().join("cache").join(&name);

        if check_file_item(&name, os, config.clone()) {
            let task = tokio::spawn(download_and_install(name, url.clone(), file_path, config.clone(), m.clone()));
            tasks.push(task);
        }
    }

    // Wait for all downloads to finish
    for task in tasks {
        let _ = task.await.expect("下载或安装失败");
    }
}

async fn download_and_install(component: String, url: String, file_path: PathBuf, config: Config, m: MultiProgress) {
    if !file_path.exists() {
        let bar_style = ProgressStyle::with_template("{prefix:.bold} {spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {binary_bytes}/{binary_total_bytes} ({binary_bytes_per_sec}, {eta})").unwrap().progress_chars("#>-");
        let pb = m.add(ProgressBar::new(100));
        pb.set_style(bar_style);
        pb.set_prefix(format!("下载 {}", component));

        if let Err(err) = download_file(url.clone(), file_path.clone(), |len, total| {
            pb.set_length(total);
            pb.inc(len as u64);
        }).await {
            eprintln!("Error downloading file {}: {}", file_path.display(), err);
            return;
        }
        pb.finish();
    }

    let spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
        .unwrap()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");

    let pb = m.add(ProgressBar::new_spinner());
    pb.set_prefix(format!("安装{}", component));
    pb.set_style(spinner_style);

    unzip(file_path, config.get_rime_config_path(), pb).await;

    // pb.finish_with_message("完成");
}


fn backup(backup_version: &String, config: &Config) {
    let target_path = config.working_dir.join("backup").join(backup_version);
    if target_path.exists() {
        return;
    }
    fs::create_dir_all(target_path.clone()).unwrap();
    println!("备份当前版本到：{}", target_path.display());

    let source_path = config.get_rime_config_path();
    if !source_path.exists() {
        return;
    }

    let spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
        .unwrap()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");
    let pb = ProgressBar::new(1);
    pb.set_prefix(format!("备份 {}", backup_version));
    pb.set_style(spinner_style);
    copy_dir_contents(&source_path, &target_path, |path| {
        pb.set_message(format!("{}", path.display()));
        pb.inc(1);
    }).unwrap();
    pb.finish_with_message("完成");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = cli::Cli::parse();

    let platform = cli.platform;
    let workding_dir = cli.working_dir.unwrap_or(PathBuf::from(Config::path_in_home(".sbsrf-update").join(platform.clone())));

    let mut config = config::Config::new(workding_dir);
    let local_version = config.get_version_id();

    let release = Release::init().await?;
    let release_version = release.get_id();
    let version_name = release.get_version();

    if release_version == local_version && !cli.force {
        println!("{} 上安装的已经是最新版本: {}", style(platform.clone()).cyan(), style(version_name.clone()).cyan());
    } else {
        if !cli.force{
            println!("{}", style(release.get_release_info()).green());
            println!("最新的 Release 版本 {} 已经发布", style(release.get_version()).cyan());
        }

        let force = release_version == local_version && cli.force;
        let confirmation = Confirm::new()
            .with_prompt(if force { "本地已经是最新版本，是否要重新升级？" } else { "是否要升级到最新版本？" })
            .default(true)
            .interact()
            .unwrap();
    
        if confirmation {
            if config.get_max_backups() > 0 {
                backup(&config.get_version_name(), &config);
            }

            upgrade(release, config.clone(), &platform).await;

            config.set_version(release_version, version_name);
            config.save();
        }
    }

    Ok(())
}