use reqwest::Client;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::{Command, Stdio};
// use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use futures_util::{SinkExt, StreamExt};

/// 通过 CDP (Chrome DevTools Protocol) 从运行的浏览器提取 cookies
/// 无需关闭浏览器，通过启动一个 headless 调试实例来读取 cookie 数据库
pub async fn extract_cookies_cdp(browser: &str) -> Result<String, String> {
    let (browser_exe, user_data_dir) = find_browser_paths(browser)?;

    // 找一个空闲端口
    let port = find_free_port()?;

    // 启动 headless 浏览器实例用于调试
    let mut child = Command::new(&browser_exe)
        .args([
            format!("--remote-debugging-port={}", port),
            "--headless=new".to_string(),
            format!("--user-data-dir={}", user_data_dir),
            "--no-first-run".to_string(),
            "--no-default-browser-check".to_string(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("无法启动 {} 调试实例: {}", browser, e))?;

    let _child_pid = child.id();

    // 等待 CDP 服务就绪
    let ws_url = wait_for_cdp(port, 15000).await?;

    // 通过 WebSocket 提取 cookies
    let cookies = get_cookies_via_cdp(&ws_url).await?;

    // 关闭 headless 实例
    let _ = child.kill();
    let _ = child.wait();

    // 写入 Netscape 格式 cookies 文件
    let cookies_path = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(format!("cookies-{}.txt", browser));

    write_netscape_cookies(&cookies_path, &cookies)?;

    Ok(cookies_path.to_string_lossy().to_string())
}

/// 查找浏览器可执行文件路径和用户数据目录
fn find_browser_paths(browser: &str) -> Result<(String, String), String> {
    let local_appdata = std::env::var("LOCALAPPDATA").unwrap_or_default();

    match browser.to_lowercase().as_str() {
        "chrome" => {
            let exe = find_chrome_exe(&local_appdata, "Google", "Chrome")?;
            let data = format!("{}\\Google\\Chrome\\User Data", local_appdata);
            Ok((exe, data))
        }
        "edge" => {
            let exe = find_chrome_exe(&local_appdata, "Microsoft", "Edge")?;
            let data = format!("{}\\Microsoft\\Edge\\User Data", local_appdata);
            Ok((exe, data))
        }
        "brave" => {
            let exe = find_chrome_exe(&local_appdata, "BraveSoftware", "Brave-Browser")?;
            let data = format!("{}\\BraveSoftware\\Brave-Browser\\User Data", local_appdata);
            Ok((exe, data))
        }
        "chromium" => {
            let exe = find_chrome_exe(&local_appdata, "Chromium", "Application")?;
            let data = format!("{}\\Chromium\\User Data", local_appdata);
            Ok((exe, data))
        }
        "opera" => {
            let roaming = std::env::var("APPDATA").unwrap_or_default();
            let exe = find_chrome_exe(&roaming, "Opera Software", "Opera Stable")?;
            let data = format!("{}\\Opera Software\\Opera Stable", roaming);
            Ok((exe, data))
        }
        "vivaldi" => {
            let exe = find_chrome_exe(&local_appdata, "Vivaldi", "Application")?;
            let data = format!("{}\\Vivaldi\\User Data", local_appdata);
            Ok((exe, data))
        }
        "firefox" => {
            // Firefox 不使用 Chrome CDP，回退到 yt-dlp 方式
            Err("Firefox 请使用 yt-dlp 方式提取".to_string())
        }
        _ => Err(format!("不支持的浏览器: {}", browser)),
    }
}

/// 查找 Chrome 系浏览器可执行文件
fn find_chrome_exe(base: &str, vendor: &str, product: &str) -> Result<String, String> {
    let paths = [
        format!("{}\\{}\\Application\\{}.exe", base, vendor, product.to_lowercase()),
        format!("C:\\Program Files\\{}\\Application\\{}.exe", vendor, product.to_lowercase()),
        format!("C:\\Program Files (x86)\\{}\\Application\\{}.exe", vendor, product.to_lowercase()),
    ];

    for p in &paths {
        if std::path::Path::new(p).exists() {
            return Ok(p.clone());
        }
    }
    Err(format!("找不到 {} 浏览器可执行文件", product))
}

/// 查找空闲 TCP 端口
fn find_free_port() -> Result<u16, String> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("无法绑定端口: {}", e))?;
    let port = listener.local_addr()
        .map_err(|e| format!("无法获取端口: {}", e))?
        .port();
    drop(listener);
    Ok(port)
}

/// 等待 CDP 服务就绪，返回 WebSocket URL
async fn wait_for_cdp(port: u16, timeout_ms: u64) -> Result<String, String> {
    let client = Client::new();
    let url = format!("http://127.0.0.1:{}/json/version", port);
    let start = std::time::Instant::now();

    loop {
        if start.elapsed().as_millis() > timeout_ms as u128 {
            return Err("CDP 调试服务启动超时".to_string());
        }

        match client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    let json: Value = resp.json().await.map_err(|e| format!("解析失败: {}", e))?;
                    let ws_url = json["webSocketDebuggerUrl"]
                        .as_str()
                        .ok_or("未找到 WebSocket URL")?
                        .to_string();
                    return Ok(ws_url);
                }
            }
            Err(_) => {}
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
}

/// 通过 CDP WebSocket 获取 cookies
async fn get_cookies_via_cdp(ws_url: &str) -> Result<Vec<CookieEntry>, String> {
    let (ws_stream, _) = connect_async(ws_url)
        .await
        .map_err(|e| format!("WebSocket 连接失败: {}", e))?;

    let (mut write, mut read) = ws_stream.split();

    // 获取所有 cookies（不限于特定域名）
    let cmd = json!({
        "id": 1,
        "method": "Network.getAllCookies",
    });

    write
        .send(tokio_tungstenite::tungstenite::Message::Text(cmd.to_string().into()))
        .await
        .map_err(|e| format!("发送命令失败: {}", e))?;

    let mut cookies = Vec::new();
    let mut msg_id = 2;

    while let Some(msg) = read.next().await {
        let msg = msg.map_err(|e| format!("WebSocket 错误: {}", e))?;
        if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
            let response: Value = serde_json::from_str(&text).unwrap_or_default();
            if response["id"] == 1 {
                if let Some(arr) = response["result"]["cookies"].as_array() {
                    for c in arr {
                        cookies.push(CookieEntry {
                            domain: c["domain"].as_str().unwrap_or("").to_string(),
                            name: c["name"].as_str().unwrap_or("").to_string(),
                            value: c["value"].as_str().unwrap_or("").to_string(),
                            path: c["path"].as_str().unwrap_or("/").to_string(),
                            expires: c["expires"].as_f64().unwrap_or(0.0),
                            secure: c["secure"].as_bool().unwrap_or(false),
                            http_only: c["httpOnly"].as_bool().unwrap_or(false),
                        });
                    }
                }
                break;
            }
        }
        msg_id += 1;
    }

    // 发送关闭命令
    let _ = write
        .send(tokio_tungstenite::tungstenite::Message::Text(
            json!({"id": msg_id, "method": "Browser.close"}).to_string().into(),
        ))
        .await;

    Ok(cookies)
}

#[derive(Debug, Clone)]
struct CookieEntry {
    domain: String,
    name: String,
    value: String,
    path: String,
    expires: f64,
    secure: bool,
    http_only: bool,
}

/// 写入 Netscape 格式 cookies 文件
fn write_netscape_cookies(path: &std::path::Path, cookies: &[CookieEntry]) -> Result<(), String> {
    let mut content = String::from("# Netscape HTTP Cookie File\n");
    content.push_str("# This file was generated by yt-dlp-ui\n");
    content.push_str("#\n\n");

    for c in cookies {
        let domain = if c.domain.starts_with('.') {
            c.domain.clone()
        } else {
            format!(".{}", c.domain)
        };

        let secure = if c.secure { "TRUE" } else { "FALSE" };
        let expires = if c.expires > 0.0 {
            c.expires as i64
        } else {
            0
        };

        content.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            domain,
            if domain.starts_with('.') { "TRUE" } else { "FALSE" },
            c.path,
            secure,
            expires,
            c.name,
            c.value,
        ));
    }

    std::fs::write(path, content)
        .map_err(|e| format!("写入 cookies 文件失败: {}", e))?;

    Ok(())
}