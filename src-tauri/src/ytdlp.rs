use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;

/// 下载进度信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub task_id: String,
    pub percent: f64,
    pub speed: String,
    pub eta: String,
    pub total_bytes: String,
    pub downloaded_bytes: String,
}

/// 执行下载任务，通过 Tauri 事件推送进度
/// 使用 std::process::Command + spawn_blocking 读取 stdout（yt-dlp 进度在 stdout）
pub async fn run_download(
    task_id: &str,
    url: &str,
    format_id: &str,
    output_dir: &str,
    ytdlp_path: &str,
    app_handle: tauri::AppHandle,
    cancel_flag: Arc<AtomicBool>,
    pid_holder: Arc<std::sync::Mutex<Option<u32>>>,
) -> Result<String, String> {
    let ytdlp = ytdlp_path.to_string();
    let url_owned = url.to_string();
    let fmt = format_id.to_string();
    let out = output_dir.to_string();
    let tid = task_id.to_string();

    let result = tokio::task::spawn_blocking(move || {
        let mut child = Command::new(&ytdlp)
            .args([
                &url_owned,
                "-f", &fmt,
                "-o", &format!("{}/%(title)s.%(ext)s", out),
                "--no-playlist", "--newline",
                "--extractor-args", "youtube:player_client=android,ios",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("无法启动下载进程: {}", e))?;

        // 存储 PID，供外部 cancel/pause 杀进程
        *pid_holder.lock().unwrap() = Some(child.id());

        let stdout = child.stdout.take().ok_or("无法获取 stdout")?;
        let stderr = child.stderr.take().ok_or("无法获取 stderr")?;

        // 读取 stderr 到另一个线程
        let stderr_handle = std::thread::spawn(move || {
            BufReader::new(stderr).lines()
                .filter_map(|l| l.ok())
                .collect::<Vec<_>>()
        });

        // 主线程读取 stdout
        let reader = BufReader::new(stdout);
        let mut last_filepath = String::new();

        for line in reader.lines() {
            if cancel_flag.load(Ordering::Relaxed) {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stderr_handle.join();
                return Err("已取消".to_string());
            }

            let line = line.unwrap_or_default();

            if line.contains("[download] Destination:") {
                last_filepath = line
                    .replace("[download] Destination:", "")
                    .trim().to_string();
            }

            if let Some(progress) = parse_download_progress(&line, &tid) {
                let _ = app_handle.emit("download-progress", &progress);
            }
        }

        let status = child.wait().map_err(|e| format!("等待进程失败: {}", e))?;
        let stderr_output = stderr_handle.join().unwrap_or_default();

        if !status.success() {
            let err_msg = if !stderr_output.is_empty() {
                stderr_output.join("\n")
            } else {
                format!("退出码: {:?}", status.code())
            };
            return Err(format!("下载失败: {}", err_msg));
        }

        if last_filepath.is_empty() {
            last_filepath = "下载完成（未能获取文件路径）".to_string();
        }
        Ok(last_filepath)
    })
    .await
    .map_err(|e| format!("spawn_blocking 失败: {}", e))?;

    result
}

/// 解析 yt-dlp 默认进度行: [download]  XX.X% of ~XXX at XXX/s ETA XX:XX
fn parse_download_progress(line: &str, task_id: &str) -> Option<DownloadProgress> {
    if !line.contains("[download]") {
        return None;
    }
    // 跳过非进度行
    if line.contains("Destination:") || line.contains("has already been downloaded") {
        return None;
    }

    let after_download = line.split("[download]").nth(1)?;
    let trimmed = after_download.trim();

    // 格式: "XX.X% of ..."
    let percent_str = trimmed.split('%').next()?.trim();
    let percent = percent_str.parse::<f64>().ok()?;

    // 提取速度: "at XXX/s"
    let speed = trimmed
        .split("at")
        .nth(1)
        .and_then(|s| s.split("ETA").next())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    // 提取 ETA: "ETA XX:XX"
    let eta = trimmed
        .split("ETA")
        .nth(1)
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    // 提取总大小: "of ~XXX "
    let total_bytes = trimmed
        .split("of")
        .nth(1)
        .and_then(|s| s.split("at").next())
        .map(|s| s.trim().trim_start_matches('~').to_string())
        .unwrap_or_default();

    Some(DownloadProgress {
        task_id: task_id.to_string(),
        percent,
        speed,
        eta,
        total_bytes,
        downloaded_bytes: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_progress_zero() {
        let line = "[download]   0.0% of  341.09MiB at  138.49KiB/s ETA 42:01";
        let result = parse_download_progress(line, "test-id");
        assert!(result.is_some(), "Should parse 0.0% progress");
        let p = result.unwrap();
        assert_eq!(p.percent, 0.0);
        assert_eq!(p.speed, "138.49KiB/s");
        assert_eq!(p.eta, "42:01");
        assert_eq!(p.total_bytes, "341.09MiB");
    }

    #[test]
    fn test_parse_progress_mid() {
        let line = "[download]  50.5% of  341.09MiB at    2.50MiB/s ETA 01:10";
        let result = parse_download_progress(line, "test-id");
        assert!(result.is_some(), "Should parse 50.5% progress");
        let p = result.unwrap();
        assert_eq!(p.percent, 50.5);
        assert_eq!(p.speed, "2.50MiB/s");
        assert_eq!(p.eta, "01:10");
        assert_eq!(p.total_bytes, "341.09MiB");
    }

    #[test]
    fn test_parse_progress_done() {
        let line = "[download] 100.0% of  341.09MiB at    5.00MiB/s ETA 00:00";
        let result = parse_download_progress(line, "test-id");
        assert!(result.is_some(), "Should parse 100% progress");
        let p = result.unwrap();
        assert_eq!(p.percent, 100.0);
    }

    #[test]
    fn test_skip_destination() {
        let line = "[download] Destination: C:\\test.mp4";
        let result = parse_download_progress(line, "test-id");
        assert!(result.is_none(), "Should skip Destination line");
    }

    #[test]
    fn test_skip_non_download() {
        let line = "[youtube] Extracting URL: https://...";
        let result = parse_download_progress(line, "test-id");
        assert!(result.is_none(), "Should skip non-download line");
    }

    #[test]
    fn test_parse_no_tilde() {
        let line = "[download]  10.0% of 100.00MiB at    1.00MiB/s ETA 01:30";
        let result = parse_download_progress(line, "test-id");
        assert!(result.is_some(), "Should parse without tilde");
        let p = result.unwrap();
        assert_eq!(p.total_bytes, "100.00MiB");
    }
}