use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::{Mutex, Semaphore};

use crate::ytdlp;

/// 下载任务
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTask {
    pub id: String,
    pub url: String,
    pub title: String,
    pub format_id: String,
    pub status: TaskStatus,
    pub progress: f64,
    pub speed: String,
    pub eta: String,
    pub filesize: String,
    pub error: Option<String>,
    pub filepath: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskStatus {
    Queued,
    Downloading,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// 下载队列管理器
pub struct DownloadQueue {
    pub tasks: Arc<Mutex<HashMap<String, DownloadTask>>>,
    pub order: Arc<Mutex<Vec<String>>>,
    pub semaphore: Arc<Semaphore>,
    pub cancel_flags: Arc<Mutex<HashMap<String, Arc<Mutex<bool>>>>>,
    pub ytdlp_path: String,
    pub output_dir: String,
}

impl DownloadQueue {
    pub fn new(max_concurrent: usize, ytdlp_path: String, output_dir: String) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            order: Arc::new(Mutex::new(Vec::new())),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            cancel_flags: Arc::new(Mutex::new(HashMap::new())),
            ytdlp_path,
            output_dir,
        }
    }

    /// 获取所有任务（按顺序）
    pub async fn get_all(&self) -> Vec<DownloadTask> {
        let tasks = self.tasks.lock().await;
        let order = self.order.lock().await;
        order.iter().filter_map(|id| tasks.get(id).cloned()).collect()
    }

    /// 添加任务到队列
    pub async fn enqueue(&self, task: DownloadTask) {
        self.order.lock().await.push(task.id.clone());
        self.tasks.lock().await.insert(task.id.clone(), task);
    }

    /// 开始处理队列（后台运行）
    pub fn process(
        &self,
        app_handle: tauri::AppHandle,
        queue: Arc<DownloadQueue>,
    ) {
        let queue_ref = queue.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                // 找到第一个 queued 任务
                let next_id = {
                    let tasks = queue_ref.tasks.lock().await;
                    let order = queue_ref.order.lock().await;
                    order
                        .iter()
                        .find(|id| {
                            tasks
                                .get(*id)
                                .map(|t| t.status == TaskStatus::Queued)
                                .unwrap_or(false)
                        })
                        .cloned()
                };

                let Some(task_id) = next_id else {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    continue;
                };

                // 获取信号量
                let permit = queue_ref.semaphore.clone().acquire_owned().await;
                if permit.is_err() {
                    continue;
                }
                let _permit = permit.unwrap();

                // 再次检查任务状态（可能已被取消）
                {
                    let tasks = queue_ref.tasks.lock().await;
                    if let Some(t) = tasks.get(&task_id) {
                        if t.status != TaskStatus::Queued {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }

                // 更新状态为 downloading
                {
                    let mut tasks = queue_ref.tasks.lock().await;
                    if let Some(t) = tasks.get_mut(&task_id) {
                        t.status = TaskStatus::Downloading;
                        t.progress = 0.0;
                    }
                }
                let _ = app_handle.emit("task-updated", &task_id);

                // 获取任务信息
                let (url, format_id) = {
                    let tasks = queue_ref.tasks.lock().await;
                    let t = tasks.get(&task_id).unwrap();
                    (t.url.clone(), t.format_id.clone())
                };

                let cancel_flag = {
                    let mut flags = queue_ref.cancel_flags.lock().await;
                    let flag = Arc::new(Mutex::new(false));
                    flags.insert(task_id.clone(), flag.clone());
                    flag
                };

                // 执行下载
                let result = ytdlp::run_download(
                    &task_id,
                    &url,
                    &format_id,
                    &queue_ref.output_dir,
                    &queue_ref.ytdlp_path,
                    app_handle.clone(),
                    &cancel_flag,
                )
                .await;

                // 清理取消标志
                {
                    let mut flags = queue_ref.cancel_flags.lock().await;
                    flags.remove(&task_id);
                }

                // 更新最终状态
                {
                    let mut tasks = queue_ref.tasks.lock().await;
                    if let Some(t) = tasks.get_mut(&task_id) {
                        match result {
                            Ok(filepath) => {
                                t.status = TaskStatus::Completed;
                                t.progress = 100.0;
                                t.filepath = Some(filepath);
                            }
                            Err(e) => {
                                if e.contains("已取消") {
                                    t.status = TaskStatus::Cancelled;
                                } else {
                                    t.status = TaskStatus::Failed;
                                    t.error = Some(e);
                                }
                            }
                        }
                    }
                }
                let _ = app_handle.emit("task-updated", &task_id);
            }
        });
    }

    /// 暂停任务
    pub async fn pause(&self, task_id: &str) -> Result<(), String> {
        let mut tasks = self.tasks.lock().await;
        if let Some(t) = tasks.get_mut(task_id) {
            if t.status == TaskStatus::Downloading {
                t.status = TaskStatus::Paused;
                // 设置取消标志来停止下载
                let flags = self.cancel_flags.lock().await;
                if let Some(flag) = flags.get(task_id) {
                    let mut f = flag.lock().await;
                    *f = true;
                }
                return Ok(());
            }
        }
        Err("任务未找到或状态不正确".to_string())
    }

    /// 取消任务
    pub async fn cancel(&self, task_id: &str) -> Result<(), String> {
        {
            let mut tasks = self.tasks.lock().await;
            if let Some(t) = tasks.get_mut(task_id) {
                if t.status == TaskStatus::Completed || t.status == TaskStatus::Cancelled {
                    return Err("任务已完成或已取消".to_string());
                }
                t.status = TaskStatus::Cancelled;
            } else {
                return Err("任务未找到".to_string());
            }
        }

        let flags = self.cancel_flags.lock().await;
        if let Some(flag) = flags.get(task_id) {
            let mut f = flag.lock().await;
            *f = true;
        }
        Ok(())
    }

    /// 标记任务为排队中
    pub async fn requeue(&self, task_id: &str) -> Result<(), String> {
        let mut tasks = self.tasks.lock().await;
        if let Some(t) = tasks.get_mut(task_id) {
            if t.status == TaskStatus::Paused || t.status == TaskStatus::Failed {
                t.status = TaskStatus::Queued;
                return Ok(());
            }
        }
        Err("无法重新排队".to_string())
    }
}