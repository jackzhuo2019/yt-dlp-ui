// 独立测试：验证 yt-dlp 下载和进度解析
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[tokio::main]
async fn main() {
    let ytdlp_path = r"D:\DEEPHARNESS\yt-dlp-ui\yt-dlp.exe";
    let url = "https://www.youtube.com/watch?v=Lf5oqGOCRCM";
    let format_id = "18";
    let output_dir = std::env::temp_dir().to_string_lossy().to_string();

    println!("=== 测试下载流程 ===");
    println!("URL: {}", url);
    println!("Format: {}", format_id);
    println!("Output: {}", output_dir);

    let mut child = Command::new(ytdlp_path)
        .args([
            url,
            "-f",
            format_id,
            "-o",
            &format!("{}/test-dl.%(ext)s", output_dir),
            "--no-playlist",
            "--newline",
            "--extractor-args",
            "youtube:player_client=android,ios",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("Failed to spawn yt-dlp");

    let stderr = child.stderr.take().expect("No stderr");
    let mut reader = BufReader::new(stderr).lines();
    let mut progress_count = 0;

    while let Ok(Some(line)) = reader.next_line().await {
        println!("[STDERR] {}", line);

        // 测试进度解析
        if line.contains("[download]") && !line.contains("Destination:") {
            let after = line.split("[download]").nth(1).unwrap().trim();
            if let Some(percent_str) = after.split('%').next() {
                if let Ok(percent) = percent_str.trim().parse::<f64>() {
                    let speed = after.split("at").nth(1)
                        .and_then(|s| s.split("ETA").next())
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();
                    let eta = after.split("ETA").nth(1)
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();
                    progress_count += 1;
                    println!("  -> Progress: {}% | speed: {} | ETA: {}", percent, speed, eta);
                }
            }
        }
    }

    let status = child.wait().await.expect("Failed to wait");
    println!("\n=== 结果 ===");
    println!("Exit code: {:?}", status.code());
    println!("Progress events: {}", progress_count);
    println!("Success: {}", status.success() && progress_count > 0);
}