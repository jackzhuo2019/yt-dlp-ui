use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 下载历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecord {
    pub id: String,
    pub url: String,
    pub title: String,
    pub format_id: String,
    pub ext: String,
    pub resolution: String,
    pub filesize: String,
    pub filepath: String,
    pub downloaded_at: String,
}

/// 历史记录存储（JSON 文件）
pub struct HistoryStore {
    path: PathBuf,
    records: Vec<HistoryRecord>,
}

impl HistoryStore {
    /// 加载历史记录
    pub fn load(app_data_dir: &PathBuf) -> Self {
        let path = app_data_dir.join("history.json");
        let records = if path.exists() {
            let content = fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        };

        Self { path, records }
    }

    /// 保存历史记录
    fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
        }
        let content = serde_json::to_string_pretty(&self.records)
            .map_err(|e| format!("序列化失败: {}", e))?;
        fs::write(&self.path, content).map_err(|e| format!("写入失败: {}", e))?;
        Ok(())
    }

    /// 添加下载记录
    pub fn add(
        &mut self,
        url: &str,
        title: &str,
        format_id: &str,
        ext: &str,
        resolution: &str,
        filesize: &str,
        filepath: &str,
    ) -> Result<HistoryRecord, String> {
        let record = HistoryRecord {
            id: uuid::Uuid::new_v4().to_string(),
            url: url.to_string(),
            title: title.to_string(),
            format_id: format_id.to_string(),
            ext: ext.to_string(),
            resolution: resolution.to_string(),
            filesize: filesize.to_string(),
            filepath: filepath.to_string(),
            downloaded_at: Utc::now().to_rfc3339(),
        };
        self.records.push(record.clone());
        self.save()?;
        Ok(record)
    }

    /// 获取所有历史记录（按时间倒序）
    pub fn get_all(&self) -> Vec<HistoryRecord> {
        let mut records = self.records.clone();
        records.reverse();
        records
    }

    /// 搜索历史记录
    pub fn search(&self, query: &str) -> Vec<HistoryRecord> {
        let q = query.to_lowercase();
        self.records
            .iter()
            .filter(|r| {
                r.title.to_lowercase().contains(&q) || r.url.to_lowercase().contains(&q)
            })
            .cloned()
            .rev()
            .collect()
    }

    /// 删除历史记录
    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        self.records.retain(|r| r.id != id);
        self.save()
    }

    /// 检查 URL 是否已下载过
    pub fn has_url(&self, url: &str) -> bool {
        self.records.iter().any(|r| r.url == url)
    }
}