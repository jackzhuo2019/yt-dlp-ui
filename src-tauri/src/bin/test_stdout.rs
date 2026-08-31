// 测试 v3：读取 stdout 而非 stderr
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

fn main() {
    let ytdlp_path = r"D:\DEEPHARNESS\yt-dlp-ui\yt-dlp.exe";
    let url = "https://www.youtube.com/watch?v=Lf5oqGOCRCM";
    let output_dir = std::env::temp_dir().to_string_lossy().to_string();

    println!("=== 测试 stdout ===");
    let mut child = Command::new(ytdlp_path)
        .args([
            url, "-f", "18",
            "-o", &format!("{}/test-stdout.%(ext)s", output_dir),
            "--no-playlist", "--newline",
            "--extractor-args", "youtube:player_client=android,ios",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn().expect("spawn");

    // 读 stdout
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    // 在另一个线程读 stderr
    let stderr_handle = std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            eprintln!("[STDERR] {}", line.unwrap());
        }
    });

    println!("--- stdout ---");
    let reader = BufReader::new(stdout);
    let mut count = 0;
    for line in reader.lines() {
        let line = line.unwrap();
        println!("[STDOUT] {}", line);
        count += 1;
        if count >= 5 { child.kill().ok(); break; }
    }

    stderr_handle.join().ok();
    let _ = child.wait();
    println!("stdout lines: {}", count);
}