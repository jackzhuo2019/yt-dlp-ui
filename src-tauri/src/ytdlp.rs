use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;

/// 视频格式信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Format {
    pub format_id: String,
    pub ext: String,
    pub resolution: String,
    pub fps: Option<f64>,
    pub filesize: Option<u64>,
    pub vcodec: String,
    pub acodec: String,
    pub format_note: String,
}

/// yt-dlp -J 输出的原始 JSON 中单个格式的结构
#[derive(Debug, Deserialize)]
struct RawFormat {
    format_id: String,
    ext: String,
    resolution: Option<String>,
    fps: Option<f64>,
    filesize: Option<u64>,
    vcodec: Option<String>,
    acodec: Option<String>,
    format_note: Option<String>,
}

/// yt-dlp -J 输出的顶层 JSON
#[derive(Debug, Deserialize)]
struct VideoInfo {
    title: String,
    formats: Vec<RawFormat>,
}

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

/// 获取视频信息和可用格式列表
pub async fn fetch_formats(url: &str, ytdlp_path: &str) -> Result<(String, Vec<Format>), String> {
    let output = Command::new(ytdlp_path)
        .args(["-J", "--no-playlist", "--extractor-args", "youtube:player_client=android,ios", url])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("无法启动 yt-dlp: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("yt-dlp 错误: {}", stderr));
    }

    let info: VideoInfo =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("解析 JSON 失败: {}", e))?;

    let formats: Vec<Format> = info
        .formats
        .into_iter()
        .map(|f| Format {
            format_id: f.format_id,
            ext: f.ext,
            resolution: f.resolution.unwrap_or_default(),
            fps: f.fps,
            filesize: f.filesize,
            vcodec: f.vcodec.unwrap_or_default(),
            acodec: f.acodec.unwrap_or_default(),
            format_note: f.format_note.unwrap_or_default(),
        })
        .collect();

    Ok((info.title, formats))
}

/// 执行下载任务，通过 Tauri 事件推送进度
/// yt-dlp 的下载进度和 Destination 都在 stderr
pub async fn run_download(
    task_id: &str,
    url: &str,
    format_id: &str,
    output_dir: &str,
    ytdlp_path: &str,
    app_handle: tauri::AppHandle,
    cancel_flag: &Mutex<bool>,
) -> Result<String, String> {
    let mut child = Command::new(ytdlp_path)
        .args([
            url,
            "-f",
            format_id,
            "-o",
            &format!("{}/%(title)s.%(ext)s", output_dir),
            "--no-playlist",
            "--newline",
            "--extractor-args",
            "youtube:player_client=android,ios",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("无法启动下载进程: {}", e))?;

    let stdout = child.stdout.take().ok_or("无法获取 stdout")?;
    let stderr = child.stderr.take().ok_or("无法获取 stderr")?;

    // 后台读取 stderr（仅用于错误收集）
    let mut stderr_lines = Vec::new();
    let stderr_handle = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            stderr_lines.push(line);
        }
        stderr_lines
    });

    // 主线程读取 stdout（进度在这里）
    let mut reader = BufReader::new(stdout).lines();
    let mut last_filepath = String::new();

    while let Ok(Some(line)) = reader.next_line().await {
        // 检查取消标志
        {
            let cancelled = cancel_flag.lock().await;
            if *cancelled {
                child.kill().await.ok();
                stderr_handle.abort();
                return Err("任务已取消".to_string());
            }
        }

        // 捕获目标文件路径
        if line.contains("[download] Destination:") {
            last_filepath = line
                .replace("[download] Destination:", "")
                .trim()
                .to_string();
        }

        // 解析进度: [download]   XX.X% of ~XXX at XXX/s ETA XX:XX
        if let Some(progress) = parse_download_progress(&line, task_id) {
            let _ = app_handle.emit("download-progress", &progress);
        }
    }

    let status = child.wait().await.map_err(|e| format!("等待进程失败: {}", e))?;

    // 收集 stderr 错误信息
    let stderr_output = stderr_handle.await.unwrap_or_default();

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