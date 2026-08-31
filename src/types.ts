// 视频格式
export interface Format {
  formatId: string;
  ext: string;
  resolution: string;
  fps: number | null;
  filesize: number | null;
  vcodec: string;
  acodec: string;
  formatNote: string;
}

// 下载任务状态
export type TaskStatus =
  | "queued"
  | "downloading"
  | "paused"
  | "completed"
  | "failed"
  | "cancelled";

// 下载任务
export interface DownloadTask {
  id: string;
  url: string;
  title: string;
  formatId: string;
  outputDir: string;
  status: TaskStatus;
  progress: number;
  speed: string;
  eta: string;
  filesize: string;
  error: string | null;
  filepath: string | null;
}

// 下载进度事件
export interface DownloadProgress {
  taskId: string;
  percent: number;
  speed: string;
  eta: string;
  totalBytes: string;
  downloadedBytes: string;
}

// 历史记录
export interface HistoryRecord {
  id: string;
  url: string;
  title: string;
  formatId: string;
  ext: string;
  resolution: string;
  filesize: string;
  filepath: string;
  downloadedAt: string;
}

// 格式查询结果
export interface FormatResult {
  title: string;
  formats: Format[];
}