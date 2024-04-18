use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use reqwest::Client;
use std::collections::VecDeque;
use std::env::consts::OS;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use zip::ZipArchive;
use std::process::Command;

#[cfg(target_os = "macos")]
use std::process::Stdio;

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

pub async fn unzip(file_path: &PathBuf, output_dir: &PathBuf, pb: ProgressBar) {
    let file = File::open(&file_path).unwrap();
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
    file_path: PathBuf,
    device_host: String,
    pb: ProgressBar,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let mut stack = VecDeque::new();
    stack.push_back(file_path.clone());

    let base = file_path.as_path();
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
    let home = if OS.to_string() == "windows".to_string() {
        std::env::var("USERPROFILE").unwrap()
    } else {
        std::env::var("HOME").unwrap()
    };

    PathBuf::from(home).join(".sbsrf-update")
}

pub fn get_bar_style() -> ProgressStyle {
    let template = "{prefix:.bold} {spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {binary_bytes}/{binary_total_bytes} ({binary_bytes_per_sec}, {eta})";
    ProgressStyle::with_template(&template)
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

    let backups = fs::read_dir(backup_path)
        .unwrap()
        .filter_map(Result::ok);
    let count = backups.count();
    if count >= max_backups as usize {
        let backups = fs::read_dir(backup_path)
            .unwrap()
            .filter_map(Result::ok);
        let mut backup_items: Vec<_> = backups.collect();
        backup_items.sort_by_key(|x| x.file_name());
        backup_items
            .iter()
            .take(count + 1 - max_backups as usize)
            .for_each(|backup| fs::remove_dir_all(backup.path()).unwrap())
    }
}

#[cfg(target_os = "macos")]
pub fn get_rime_home() -> Option<PathBuf> {
    let ps = Command::new("ps")
        .arg("aux")
        .stdout(Stdio::piped())
        .spawn()
        .expect("查找 Squirrel 进程失败");

    let grep = Command::new("grep")
        .arg("[S]quirrel")
        .stdin(ps.stdout.unwrap())
        .stdout(Stdio::piped())
        .spawn()
        .expect("查找 Squirrel 进程失败");

    let output = Command::new("cut")
        .args(["-f", "11-"])
        .stdin(grep.stdout.unwrap())
        .output()
        .expect("查找 Squirrel 进程失败");

    let output_str = String::from_utf8(output.stdout).unwrap();
    let splited: Vec<&str> = output_str.split_whitespace().collect();
    if splited.len() > 10 {
        let command = splited[10..].join(" ");
        Some(PathBuf::from(command).parent().unwrap().to_path_buf())
    } else {
        Option::None
    }
}

#[cfg(target_os = "windows")]
pub fn get_rime_home() -> Option<PathBuf> {
    let pid = check_weasel_server_state();
    if pid > 0 {
        Some(get_weasel_home(pid))
    } else {
        Option::None
    }
}

#[cfg(target_os = "windows")]
pub fn check_weasel_server_state() -> i32 {
    let output = Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq WeaselServer.exe"])
        .output()
        .expect("检测小狼毫程序运行状态失败");
    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{}", output_str);

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

#[cfg(target_os = "windows")]
fn get_weasel_home(process_id: i32) -> PathBuf {
    let arg = format!("ProcessId={process_id}");
    let output = Command::new("wmic")
        .args(&["process", "where", arg.as_str(), "get", "ExecutablePath"])
        .output()
        .expect("获取小狼毫安装路径失败");

    let output_str = String::from_utf8_lossy(&output.stdout);
    println!("{}", output_str);
    // ExecutablePath
    // C:\Program Files (x86)\Rime\weasel-0.14.3\WeaselServer.exe
    let mut splited = output_str.split("\r\n");
    if let Some(path) = splited.nth(1) {
        PathBuf::from(path).parent().unwrap().to_path_buf()
    } else {
        PathBuf::new()
    }
}

/**
 * 启动或关闭 WeaselServer
 */
#[cfg(target_os = "windows")]
pub fn toggle_weasel_server_state(weasel_home: PathBuf, start: bool) {
    let mut cmd = Command::new(weasel_home.join("WeaselServer.exe") .as_os_str());
    if !start {
        cmd.arg("/q");
    }
    cmd.spawn().unwrap();
}

/**
 * 重新部署
 */
#[cfg(target_os = "windows")]
pub fn deploy(weasel_home: Option<PathBuf>) {
    if let Some(home) = weasel_home {
        let mut cmd = Command::new(home.join("WeaselDeployer.exe").as_os_str());
        cmd.spawn().unwrap();
    }
}

#[cfg(target_os = "macos")]
pub fn deploy(rime_home: Option<PathBuf>) {
    println!("重新部署：{:?}", rime_home);
    if let Some(home) = rime_home {
        let output = Command::new(home.join("Squirrel").as_os_str())
            .arg("--reload")
            .output().expect("部署失败");
        println!("{}", String::from_utf8_lossy(&output.stdout))
    }
}