// 独立测试 v2：使用 std::process 而非 tokio::process
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

fn main() {
    let ytdlp_path = r"D:\DEEPHARNESS\yt-dlp-ui\yt-dlp.exe";
    let url = "https://www.youtube.com/watch?v=Lf5oqGOCRCM";
    let output_dir = std::env::temp_dir().to_string_lossy().to_string();

    println!("=== 测试下载流程 (std::process) ===");

    let mut child = Command::new(ytdlp_path)
        .args([
            url,
            "-f", "18",
            "-o", &format!("{}/test-dl3.%(ext)s", output_dir),
            "--no-playlist",
            "--newline",
            "--extractor-args", "youtube:player_client=android,ios",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn");

    let stderr = child.stderr.take().expect("No stderr");
    let reader = BufReader::new(stderr);
    let mut progress_count = 0;

    for line in reader.lines() {
        let line = line.unwrap();
        println!("[STDERR] {}", line);
        if line.contains("[download]") && !line.contains("Destination:") {
            progress_count += 1;
        }
        if progress_count >= 5 {
            // 拿到几个进度就够了，杀掉进程
            child.kill().ok();
            break;
        }
    }

    let _ = child.wait();
    println!("\n=== 结果 ===");
    println!("Progress events captured: {}", progress_count);
    println!("Success: {}", progress_count > 0);
}