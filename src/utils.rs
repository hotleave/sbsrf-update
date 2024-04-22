use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use reqwest::Client;
use std::collections::VecDeque;
use std::env::consts::OS;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use zip::ZipArchive;


use crate::error::Error;

pub fn copy_dir_contents<F>(from: &Path, to: &Path, callback: F) -> std::io::Result<()>
where
    F: Fn(&PathBuf),
{
    if !to.exists() {
        fs::create_dir_all(to).unwrap();
    }

    let mut stack = VecDeque::new();
    stack.push_back((from.to_path_buf(), to.to_path_buf()));

    while let Some((src, dst)) = stack.pop_front() {
        for entry in fs::read_dir(&src)? {
            let entry = entry?;
            let entry_path = entry.path();
            let target_path = dst.join(entry.file_name());

            if entry_path.is_dir() {
                fs::create_dir_all(&target_path)?;
                stack.push_back((entry_path, target_path));
            } else {
                callback(&entry_path);
                fs::copy(&entry_path, &target_path)?;
            }
        }
    }

    Ok(())
}

pub async fn download_file<F>(url: String, file_path: &PathBuf, callback: F) -> reqwest::Result<()>
where
    F: Fn(usize, u64),
{
    if !file_path.parent().unwrap().exists() {
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
    }
    let mut file = File::create(file_path).unwrap();
    let mut response = reqwest::get(url).await?;
    let total = response.content_length().unwrap();
    callback(0, total);
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).unwrap();
        callback(chunk.len(), total);
    }

    Ok(())
}

pub async fn unzip(file_path: &Path, output_dir: &Path, pb: &ProgressBar) {
    let file = File::open(file_path).unwrap();
    let archive = ZipArchive::new(file).unwrap();
    let file_path_arc = Arc::new(file_path);

    (0..archive.len()).into_par_iter().for_each(|i| {
        let file = File::open(file_path_arc.canonicalize().unwrap()).unwrap();
        let mut zip = ZipArchive::new(file).unwrap();
        let mut zip_file = zip.by_index(i).unwrap();
        let outpath = match zip_file.enclosed_name() {
            Some(path) => output_dir.join(path),
            None => return,
        };

        if !(*zip_file.name()).ends_with('/') {
            pb.set_message(format!(
                "Extracting file {} to {} ({} bytes)",
                zip_file.name(),
                outpath.display(),
                zip_file.size()
            ));
            pb.inc(1);

            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).unwrap();
                }
            }

            let mut outfile = File::create(&outpath).unwrap();
            std::io::copy(&mut zip_file, &mut outfile).unwrap();
        }
    });

    pb.finish_with_message("完成");
}

pub async fn upload_to_ios(
    file_path: &Path,
    device_host: &str,
    pb: &ProgressBar,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let mut stack = VecDeque::new();
    stack.push_back(file_path.to_path_buf());

    let base = file_path;
    while let Some(src) = stack.pop_front() {
        for entry in fs::read_dir(&src)? {
            let entry = entry?;
            let entry_path = entry.path();

            if entry_path.is_dir() {
                stack.push_back(entry_path);
            } else {
                let name = entry_path.as_path().strip_prefix(base)?.to_str().unwrap();
                pb.set_message(format!("上传 {name}"));
                pb.inc(1);

                let mut buffer = Vec::new();
                let mut file = File::open(entry_path.clone()).unwrap();
                file.read_to_end(&mut buffer).unwrap();

                let response = client
                    .post(format!(
                        "http://{device_host}/api/tus/Rime/{name}?override=true"
                    ))
                    .header("Content-Type", "application/octet-stream")
                    .body(buffer)
                    .send()
                    .await
                    .unwrap();
                if !response.status().is_success() {
                    eprintln!("上传失败")
                }
            }
        }
    }

    pb.finish_with_message("完成");

    Ok(())
}

pub fn work_dir() -> PathBuf {
    let home = if OS == "windows" {
        std::env::var("USERPROFILE").unwrap()
    } else {
        std::env::var("HOME").unwrap()
    };

    PathBuf::from(home).join(".sbsrf-update")
}

pub fn get_bar_style() -> ProgressStyle {
    let template = "{prefix:.bold} {spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {binary_bytes}/{binary_total_bytes} ({binary_bytes_per_sec}, {eta})";
    ProgressStyle::with_template(template)
        .unwrap()
        .progress_chars("#>-")
}

pub fn get_spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
        .unwrap()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
}

pub fn ensure_max_backups(backup_path: &PathBuf, max_backups: i32) {
    if !backup_path.exists() {
        fs::create_dir_all(backup_path).unwrap();
        return;
    }

    let backups = fs::read_dir(backup_path).unwrap().filter_map(Result::ok);
    let count = backups.count();
    if count >= max_backups as usize {
        let backups = fs::read_dir(backup_path).unwrap().filter_map(Result::ok);
        let mut backup_items: Vec<_> = backups.collect();
        backup_items.sort_by_key(|x| x.file_name());
        backup_items
            .iter()
            .take(count + 1 - max_backups as usize)
            .for_each(|backup| fs::remove_dir_all(backup.path()).unwrap())
    }
}

#[cfg(target_os = "macos")]
pub fn grep(keyword: &str) -> Result<String, Box<dyn std::error::Error>> {
    use std::process::Stdio;

    let ps = Command::new("ps")
        .arg("aux")
        .stdout(Stdio::piped())
        .spawn()
        .expect("ps 命令失败");

    let grep = Command::new("grep")
        .arg(keyword)
        .stdin(ps.stdout.unwrap())
        .stdout(Stdio::piped())
        .spawn()
        .expect("grep 命令失败");

    let tr = Command::new("tr")
        .args(["-s", " "])
        .stdin(grep.stdout.unwrap())
        .stdout(Stdio::piped())
        .spawn()
        .expect("tr 命令失败");

    let output = Command::new("cut")
        .args(["-d", " ", "-f", "11-"])
        .stdin(tr.stdout.unwrap())
        .output()
        .expect("cut 命令失败");

    let output_str = String::from_utf8(output.stdout).unwrap();
    if output_str.trim() == "" {
        return Err(Box::new(Error::new(&format!("{keyword} not found"))));
    }

    Ok(output_str.trim().to_string())
}

pub async fn download_and_install(
    target_dir: PathBuf,
    name: String,
    url: String,
    m: MultiProgress,
) {
    let cache_dir = work_dir().join("_cache");
    let file_path = cache_dir.join(&name);

    if !file_path.exists() {
        // 下载文件
        let pb = m.add(ProgressBar::new(100));
        pb.set_prefix(format!("下载 {}", &name));
        pb.set_style(get_bar_style());

        if let Err(error) = download_file(url.to_string(), &file_path, |len, total| {
            pb.set_length(total);
            pb.inc(len as u64);
        })
        .await
        {
            println!("下载文件{}失败: {error}", &name);
        }

        pb.finish();
    }

    // 解压
    let pb = m.add(ProgressBar::new_spinner());
    pb.set_prefix(format!("更新 {}", &name));
    pb.set_style(get_spinner_style());
    unzip(&file_path, &target_dir, &pb).await;
}

#[cfg(target_os = "macos")]
pub fn open(target: PathBuf) {
    println!("Open {}", target.display());
    Command::new("open")
        .arg(target.as_os_str())
        .status()
        .expect("打开文件失败");
}

#[cfg(target_os = "windows")]
pub fn open(target: PathBuf) {
    println!("Open {}", target.display());
    Command::new("cmd")
        .args(["/C", "start", target.to_str().unwrap_or_default()])
        .status()
        .expect("打开文件失败");
}
