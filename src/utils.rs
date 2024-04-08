use std::collections::VecDeque;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use indicatif::ProgressBar;
use zip::ZipArchive;
use rayon::prelude::*;

pub fn copy_dir_contents<F>(from: &PathBuf, to: &PathBuf, callback: F) -> std::io::Result<()>
where F: Fn(&PathBuf) {
    if !to.exists() {
        fs::create_dir_all(to.clone()).unwrap();
    }

    let mut stack = VecDeque::new();
    stack.push_back((from.clone(), to.clone()));

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

pub async fn download_file<F>(url: String, file_path: PathBuf, callback: F) -> reqwest::Result<()> where F: Fn(usize, u64) {
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

pub async fn unzip(file_path: PathBuf, output_dir: PathBuf, pb: ProgressBar) {
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