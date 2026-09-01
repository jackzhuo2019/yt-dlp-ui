use std::path::Path;
use rusqlite::Connection;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm,
};
use base64::Engine;

#[link(name = "crypt32")]
extern "system" {
    fn CryptUnprotectData(
        pDataIn: *const DataBlob,
        ppszDataDescr: *mut *mut u16,
        pOptionalEntropy: *const DataBlob,
        pvReserved: *const std::ffi::c_void,
        pPromptStruct: *const std::ffi::c_void,
        dwFlags: u32,
        pDataOut: *mut DataBlob,
    ) -> i32;
}

#[repr(C)]
#[allow(non_snake_case)]
struct DataBlob {
    cbData: u32,
    pbData: *mut u8,
}

#[derive(Debug)]
struct Cookie {
    domain: String,
    name: String,
    value: String,
    path: String,
    expires: i64,
    secure: bool,
}

pub fn extract_cookies(browser: &str) -> Result<String, String> {
    let (cookies_db, local_state) = find_browser_data(browser)?;

    let output_path = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join(format!("cookies-{}.txt", browser));

    let temp_dir = std::env::temp_dir().join("yt-dlp-ui-cookies");
    let _ = std::fs::create_dir_all(&temp_dir);
    let temp_db = temp_dir.join(format!("cookies-{}.db", uuid::Uuid::new_v4()));

    let cookies = match std::fs::copy(&cookies_db, &temp_db) {
        Ok(_) => {
            let c = read_cookies(&temp_db, &local_state)?;
            let _ = std::fs::remove_file(&temp_db);
            c
        }
        Err(_) => {
            let db_uri = format!("file:{}?mode=ro&immutable=1", cookies_db.replace('\\', "/"));
            let c = read_cookies_uri(&db_uri, &local_state)
                .map_err(|e| format!("{} Cookie 数据库被锁定且无法直接读取。请关闭所有浏览器窗口和后台进程后重试。\n\n{}", browser, e))?;
            c
        }
    };

    write_netscape(&output_path, &cookies)?;

    Ok(output_path.to_string_lossy().to_string())
}

fn find_browser_data(browser: &str) -> Result<(String, String), String> {
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();

    let (vendor_dir, product_dir) = match browser.to_lowercase().as_str() {
        "chrome" => ("Google", "Chrome"),
        "edge" => ("Microsoft", "Edge"),
        "brave" => ("BraveSoftware", "Brave-Browser"),
        "chromium" => ("Chromium", "Application"),
        "opera" => {
            let roaming = std::env::var("APPDATA").unwrap_or_default();
            return Ok((
                format!("{}\\Opera Software\\Opera Stable\\Network\\Cookies", roaming),
                format!("{}\\Opera Software\\Opera Stable\\Local State", roaming),
            ));
        }
        "vivaldi" => ("Vivaldi", "User Data"),
        "firefox" => return Err("Firefox 暂不支持自动提取，请手动导出 cookies。".to_string()),
        _ => return Err(format!("不支持的浏览器: {}", browser)),
    };

    let user_data = if browser.to_lowercase() == "vivaldi" {
        format!("{}\\{}\\{}", local, vendor_dir, product_dir)
    } else if browser.to_lowercase() == "chromium" {
        format!("{}\\{}\\User Data", local, vendor_dir)
    } else {
        format!("{}\\{}\\{}\\User Data", local, vendor_dir, product_dir)
    };

    let network_cookies = format!("{}\\Default\\Network\\Cookies", user_data);
    let legacy_cookies = format!("{}\\Default\\Cookies", user_data);
    let local_state = format!("{}\\Local State", user_data);

    let cookies_path = if Path::new(&network_cookies).exists() {
        network_cookies
    } else if Path::new(&legacy_cookies).exists() {
        legacy_cookies
    } else {
        return Err(format!("找不到 {} 的 Cookie 数据库文件", browser));
    };

    if !Path::new(&local_state).exists() {
        return Err(format!("找不到 {} 的 Local State 文件", browser));
    }

    Ok((cookies_path, local_state))
}

fn read_cookies(db_path: &Path, local_state: &str) -> Result<Vec<Cookie>, String> {
    let aes_key = get_aes_key(local_state);
    let conn = Connection::open(db_path)
        .map_err(|e| format!("无法打开 Cookie 数据库: {}", e))?;
    query_cookies(&conn, &aes_key)
}

fn read_cookies_uri(uri: &str, local_state: &str) -> Result<Vec<Cookie>, String> {
    let aes_key = get_aes_key(local_state);
    let conn = Connection::open_with_flags(
        uri,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|e| format!("无法以只读模式打开: {}", e))?;
    query_cookies(&conn, &aes_key)
}

fn query_cookies(conn: &Connection, aes_key: &Option<Vec<u8>>) -> Result<Vec<Cookie>, String> {
    let mut stmt = conn
        .prepare("SELECT host_key, name, value, encrypted_value, path, expires_utc, is_secure FROM cookies")
        .map_err(|e| format!("查询失败: {}", e))?;

    let rows = stmt
        .query_map([], |row| {
            let host_key: String = row.get(0)?;
            let name: String = row.get(1)?;
            let plain_value: String = row.get(2)?;
            let enc_value: Option<Vec<u8>> = row.get(3)?;
            let path: String = row.get(4)?;
            let expires_utc: i64 = row.get(5)?;
            let is_secure: bool = row.get(6)?;

            let value = if let Some(ref enc) = enc_value {
                if !enc.is_empty() {
                    if let Some(ref key) = aes_key {
                        decrypt_value(enc, key).unwrap_or_default()
                    } else {
                        String::new()
                    }
                } else {
                    plain_value
                }
            } else {
                plain_value
            };

            Ok(Cookie {
                domain: host_key,
                name,
                value,
                path,
                expires: expires_utc,
                secure: is_secure,
            })
        })
        .map_err(|e| format!("读取数据失败: {}", e))?;

    let mut cookies = Vec::new();
    for row in rows {
        cookies.push(row.map_err(|e| format!("解析行失败: {}", e))?);
    }

    Ok(cookies)
}

fn get_aes_key(local_state_path: &str) -> Option<Vec<u8>> {
    let content = std::fs::read_to_string(local_state_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let encrypted_key_b64 = json.get("os_crypt")?.get("encrypted_key")?.as_str()?;
    let encrypted_blob = base64::engine::general_purpose::STANDARD.decode(encrypted_key_b64).ok()?;
    if encrypted_blob.len() < 6 || &encrypted_blob[..5] != b"DPAPI" {
        return None;
    }
    dpapi_decrypt(&encrypted_blob[5..])
}

fn dpapi_decrypt(data: &[u8]) -> Option<Vec<u8>> {
    let input = DataBlob { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
    let mut output = DataBlob { cbData: 0, pbData: std::ptr::null_mut() };

    let result = unsafe {
        CryptUnprotectData(&input, std::ptr::null_mut(), std::ptr::null(), std::ptr::null(), std::ptr::null(), 0, &mut output)
    };

    if result == 0 || output.pbData.is_null() {
        return None;
    }

    let decrypted = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();

    unsafe {
        extern "system" { fn LocalFree(mem: *mut std::ffi::c_void) -> *mut std::ffi::c_void; }
        LocalFree(output.pbData as *mut std::ffi::c_void);
    }

    Some(decrypted)
}

fn decrypt_value(encrypted: &[u8], key: &[u8]) -> Option<String> {
    if encrypted.len() < 3 + 12 + 16 || key.len() != 32 {
        return None;
    }
    let prefix = &encrypted[..3];
    match prefix {
        b"v10" | b"v11" => {}
        _ => return None,
    };
    let nonce = aes_gcm::Nonce::from_slice(&encrypted[3..15]);
    let cipher = Aes256Gcm::new_from_slice(key).ok()?;
    let plaintext = cipher.decrypt(nonce, &encrypted[15..]).ok()?;
    String::from_utf8(plaintext).ok()
}

fn write_netscape(path: &Path, cookies: &[Cookie]) -> Result<(), String> {
    let mut content = String::from("# Netscape HTTP Cookie File\n# This file is generated by yt-dlp-ui\n#\n\n");

    for c in cookies {
        let domain = if c.domain.starts_with('.') { c.domain.clone() } else { format!(".{}", c.domain) };
        let flag = if domain.starts_with('.') { "TRUE" } else { "FALSE" };
        let secure = if c.secure { "TRUE" } else { "FALSE" };
        let expires = if c.expires > 0 {
            (c.expires / 1_000_000).saturating_sub(11_644_473_600)
        } else { 0 };

        content.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            domain, flag, c.path, secure, expires, c.name, c.value,
        ));
    }

    std::fs::write(path, content).map_err(|e| format!("写入失败: {}", e))?;
    Ok(())
}

