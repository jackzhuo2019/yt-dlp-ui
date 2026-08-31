// 测试 v4：正确的输出路径，同时读 stdout/stderr
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

fn main() {
    let ytdlp_path = r"D:\DEEPHARNESS\yt-dlp-ui\yt-dlp.exe";
    let url = "https://www.youtube.com/watch?v=Lf5oqGOCRCM";
    let out = format!("{}\\test-progress.mp4", std::env::temp_dir().display());

    // 先删除旧文件
    let _ = std::fs::remove_file(&out);

    println!("Output: {}", out);

    let mut child = Command::new(ytdlp_path)
        .args([
            url, "-f", "18",
            "-o", &out,
            "--no-playlist", "--newline",
            "--extractor-args", "youtube:player_client=android,ios",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn().expect("spawn");

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    // 读 stderr 线程
    let stderr_handle = std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            let l = line.unwrap();
            if l.contains("[download]") {
                eprintln!("[STDERR-DL] {}", l);
            }
        }
    });

    // 读 stdout
    for line in BufReader::new(stdout).lines() {
        let l = line.unwrap();
        if l.contains("[download]") {
            println!("[STDOUT-DL] {}", l);
        }
    }

    stderr_handle.join().ok();
    let status = child.wait().unwrap();
    println!("Exit: {:?}", status.code());
}