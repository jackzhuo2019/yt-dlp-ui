use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::time::Duration;
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
    cookies_from: &str,
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
    let cookies = cookies_from.to_string();

    let result = tokio::task::spawn_blocking(move || {
        let mut command = Command::new(&ytdlp);
        // 设置 PATH 让 yt-dlp 找到 deno.exe（JS 运行时）
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_default();
        let current_dir = std::env::current_dir().unwrap_or_default();
        let extra_path = format!(
            "{};{};{}",
            exe_dir.display(),
            current_dir.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        command.env("PATH", &extra_path);
        command.args([
            &url_owned,
            "-f", &fmt,
            "-o", &format!("{}/%(title)s.%(ext)s", out),
            "--no-playlist", "--newline",
            "--windows-filenames",
        ]);
        if !cookies.is_empty() {
            command.args(["--cookies-from-browser", &cookies]);
        }

        let mut last_filepath = String::new();
        let result = run_command_with_cancel(
            command,
            &cancel_flag,
            Some(&pid_holder),
            |line| {
                if line.contains("[download] Destination:") {
                    last_filepath = line
                        .replace("[download] Destination:", "")
                        .trim().to_string();
                }
                if let Some(progress) = parse_download_progress(line, &tid) {
                    let _ = app_handle.emit("download-progress", &progress);
                }
            },
        );

        match result {
            Ok(()) => {
                if last_filepath.is_empty() {
                    last_filepath = "下载完成（未能获取文件路径）".to_string();
                }
                Ok(last_filepath)
            }
            Err(e) => Err(e),
        }
    })
    .await
    .map_err(|e| format!("spawn_blocking 失败: {}", e))?;

    result
}

/// 运行命令并逐行读取 stdout，支持随时取消。
/// 关键设计：stdout 读取放在独立线程，主循环通过 channel + recv_timeout 轮询，
/// 即使进程长时间无输出（如解析阶段），取消也能在 100ms 内生效。
fn run_command_with_cancel(
    mut command: Command,
    cancel_flag: &AtomicBool,
    pid_holder: Option<&std::sync::Mutex<Option<u32>>>,
    mut on_line: impl FnMut(&str),
) -> Result<(), String> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("无法启动下载进程: {}", e))?;

    // 存储 PID，供外部 cancel/pause 杀进程（备份手段）
    if let Some(holder) = pid_holder {
        *holder.lock().unwrap() = Some(child.id());
    }

    let stdout = child.stdout.take().ok_or("无法获取 stdout")?;
    let stderr = child.stderr.take().ok_or("无法获取 stderr")?;

    // stderr 读取线程
    let stderr_handle = std::thread::spawn(move || {
        BufReader::new(stderr).lines()
            .filter_map(|l| l.ok())
            .collect::<Vec<_>>()
    });

    // stdout 读取线程：逐行发送到 channel，避免主线程阻塞在 read 上
    let (tx, rx) = mpsc::channel::<String>();
    let stdout_handle = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            match line {
                Ok(l) => {
                    if tx.send(l).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let mut cancelled = false;
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => {
                if cancel_flag.load(Ordering::Relaxed) {
                    cancelled = true;
                    break;
                }
                on_line(&line);
            }
            Err(RecvTimeoutError::Timeout) => {
                // 无输出时也要检查取消标志（这是修复卡死的关键）
                if cancel_flag.load(Ordering::Relaxed) {
                    cancelled = true;
                    break;
                }
            }
            Err(RecvTimeoutError::Disconnected) => break, // 进程结束，stdout 关闭
        }
    }

    if cancelled {
        // kill 之后管道关闭，读取线程收到 EOF 自然退出
        let _ = child.kill();
        let _ = child.wait();
        let _ = stdout_handle.join();
        let _ = stderr_handle.join();
        return Err("已取消".to_string());
    }

    let _ = stdout_handle.join();
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

    Ok(())
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
    fn test_cancel_during_output() {
        // ping 持续输出，模拟下载中；500ms 后取消
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            flag_clone.store(true, Ordering::SeqCst);
        });

        let mut cmd = Command::new("ping");
        cmd.args(["-n", "30", "127.0.0.1"]); // 约 30 秒

        let start = std::time::Instant::now();
        let mut line_count = 0;
        let result = run_command_with_cancel(cmd, &flag, None, |_| line_count += 1);
        let elapsed = start.elapsed();

        assert!(result.is_err(), "取消后应返回 Err");
        assert!(result.unwrap_err().contains("已取消"), "错误信息应为已取消");
        assert!(line_count > 0, "取消前应已收到部分输出");
        assert!(
            elapsed < Duration::from_secs(5),
            "取消应在 5 秒内生效，实际耗时 {:?}",
            elapsed
        );
    }

    #[test]
    fn test_cancel_during_silence() {
        // ping -n 1 先输出再静默等待退出；用长 sleep 进程模拟无输出阶段
        // choice 命令会等待按键，模拟长时间无输出的进程
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            flag_clone.store(true, Ordering::SeqCst);
        });

        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "timeout", "/T", "30", "/NOBREAK"]); // 30 秒无输出

        let start = std::time::Instant::now();
        let result = run_command_with_cancel(cmd, &flag, None, |_| {});
        let elapsed = start.elapsed();

        assert!(result.is_err(), "取消后应返回 Err");
        assert!(
            elapsed < Duration::from_secs(5),
            "无输出阶段取消也应在 5 秒内生效，实际耗时 {:?}",
            elapsed
        );
    }

    #[test]
    fn test_normal_completion() {
        let flag = Arc::new(AtomicBool::new(false));
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "echo", "hello"]);

        let mut lines = Vec::new();
        let result = run_command_with_cancel(cmd, &flag, None, |l| lines.push(l.to_string()));

        assert!(result.is_ok(), "正常完成应返回 Ok: {:?}", result.err());
        assert_eq!(lines, vec!["hello".to_string()]);
    }

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