mod history;
mod queue;
mod ytdlp;

use history::HistoryStore;
use queue::{DownloadQueue, DownloadTask, TaskStatus};
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

/// 应用状态
struct AppState {
    queue: Arc<DownloadQueue>,
    history: Arc<Mutex<HistoryStore>>,
}

/// 添加下载任务
#[tauri::command]
async fn add_task(
    url: String,
    title: String,
    format_id: String,
    output_dir: String,
    cookies_from: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let task = DownloadTask {
        id: uuid::Uuid::new_v4().to_string(),
        url,
        title,
        format_id,
        output_dir,
        cookies_from: cookies_from.unwrap_or_default(),
        status: TaskStatus::Queued,
        progress: 0.0,
        speed: String::new(),
        eta: String::new(),
        filesize: String::new(),
        error: None,
        filepath: None,
    };

    let id = task.id.clone();
    state.queue.enqueue(task).await;
    Ok(id)
}

/// 获取所有任务
#[tauri::command]
async fn get_tasks(state: tauri::State<'_, AppState>) -> Result<Vec<DownloadTask>, String> {
    Ok(state.queue.get_all().await)
}

/// 获取默认下载目录
#[tauri::command]
async fn get_default_output_dir(state: tauri::State<'_, AppState>) -> Result<String, String> {
    Ok(state.queue.output_dir.clone())
}

/// 暂停任务
#[tauri::command]
async fn pause_task(task_id: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.queue.pause(&task_id).await
}

/// 取消任务
#[tauri::command]
async fn cancel_task(task_id: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.queue.cancel(&task_id).await
}

/// 重新排队
#[tauri::command]
async fn requeue_task(task_id: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.queue.requeue(&task_id).await
}

/// 获取下载历史
#[tauri::command]
async fn get_history(
    query: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<history::HistoryRecord>, String> {
    let store = state.history.lock().await;
    Ok(match query {
        Some(q) if !q.is_empty() => store.search(&q),
        _ => store.get_all(),
    })
}

/// 删除历史记录
#[tauri::command]
async fn delete_history(id: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut store = state.history.lock().await;
    store.delete(&id)
}

/// 添加历史记录
#[tauri::command]
async fn add_history(
    url: String,
    title: String,
    format_id: String,
    ext: String,
    resolution: String,
    filesize: String,
    filepath: String,
    state: tauri::State<'_, AppState>,
) -> Result<history::HistoryRecord, String> {
    let mut store = state.history.lock().await;
    store.add(&url, &title, &format_id, &ext, &resolution, &filesize, &filepath)
}

/// 从浏览器提取 cookies 到文件
#[tauri::command]
async fn extract_cookies(
    browser: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let ytdlp_path = state.queue.ytdlp_path.clone();
    let cookies_path = ytdlp::extract_cookies(&browser, &ytdlp_path).await?;
    Ok(cookies_path)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // 获取 yt-dlp 路径：优先从 Cargo 清单目录（开发模式）查找
            let ytdlp_path = std::env::var("CARGO_MANIFEST_DIR")
                .map(|d| std::path::PathBuf::from(d).join("..").join("yt-dlp.exe"))
                .ok()
                .or_else(|| {
                    // 尝试从当前工作目录查找
                    std::env::current_dir()
                        .ok()
                        .map(|d| d.join("yt-dlp.exe"))
                })
                .filter(|p| p.exists())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "yt-dlp.exe".to_string());

            let output_dir = app
                .path()
                .download_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| {
                    std::env::current_dir()
                        .map(|d| d.join("downloads").to_string_lossy().to_string())
                        .unwrap_or_else(|_| "./downloads".to_string())
                });

            let app_data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));

            // 初始化历史存储
            let history_store = HistoryStore::load(&app_data_dir);

            // 初始化下载队列
            let queue = Arc::new(DownloadQueue::new(3, ytdlp_path, output_dir));

            let app_handle = app.handle().clone();
            let queue_clone = queue.clone();
            queue.process(app_handle, queue_clone);

            app.manage(AppState {
                queue,
                history: Arc::new(Mutex::new(history_store)),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            add_task,
            get_tasks,
            get_default_output_dir,
            pause_task,
            cancel_task,
            requeue_task,
            get_history,
            delete_history,
            add_history,
            extract_cookies,
        ])
        .run(tauri::generate_context!())
        .expect("启动应用失败");
}