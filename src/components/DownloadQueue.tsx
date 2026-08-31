import { useStore } from "@/store";
import type { DownloadTask } from "@/types";

export default function DownloadQueue() {
  const tasks = useStore((s) => s.tasks);

  if (tasks.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center text-gray-600">
        <div className="text-center">
          <div className="text-4xl mb-3">⬇️</div>
          <p className="text-sm">暂无下载任务</p>
          <p className="text-xs mt-1">在上方粘贴 URL 开始下载</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto px-5 py-3 space-y-2">
      {tasks.map((task) => (
        <DownloadItem key={task.id} task={task} />
      ))}
    </div>
  );
}

function DownloadItem({ task }: { task: DownloadTask }) {
  const pauseTask = useStore((s) => s.pauseTask);
  const cancelTask = useStore((s) => s.cancelTask);
  const requeueTask = useStore((s) => s.requeueTask);

  const statusConfig: Record<
    DownloadTask["status"],
    { label: string; color: string; bg: string }
  > = {
    queued: { label: "排队中", color: "text-yellow-400", bg: "bg-yellow-400/10" },
    downloading: {
      label: "下载中",
      color: "text-blue-400",
      bg: "bg-blue-400/10",
    },
    paused: { label: "已暂停", color: "text-gray-400", bg: "bg-gray-400/10" },
    completed: {
      label: "已完成",
      color: "text-green-400",
      bg: "bg-green-400/10",
    },
    failed: { label: "失败", color: "text-red-400", bg: "bg-red-400/10" },
    cancelled: {
      label: "已取消",
      color: "text-gray-500",
      bg: "bg-gray-500/10",
    },
  };

  const sc = statusConfig[task.status];

  return (
    <div
      className={`rounded-lg border p-3 transition-colors ${
        task.status === "downloading"
          ? "border-blue-800 bg-gray-900"
          : task.status === "completed"
          ? "border-green-800/50 bg-gray-900/50"
          : task.status === "failed"
          ? "border-red-800/50 bg-gray-900/50"
          : "border-gray-800 bg-gray-900/50"
      }`}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex-1 min-w-0">
          <p className="text-sm font-medium truncate">{task.title}</p>
          <p className="text-xs text-gray-600 truncate mt-0.5">{task.url}</p>
        </div>
        <span
          className={`text-xs px-2 py-0.5 rounded-full shrink-0 ${sc.color} ${sc.bg}`}
        >
          {sc.label}
        </span>
      </div>

      {/* 进度条 */}
      {(task.status === "downloading" || task.status === "paused") && (
        <div className="mt-2.5">
          <div className="w-full bg-gray-800 rounded-full h-2 overflow-hidden">
            <div
              className={`h-full rounded-full transition-all duration-300 ${
                task.status === "downloading"
                  ? "bg-blue-500 downloading-pulse"
                  : "bg-gray-500"
              }`}
              style={{ width: `${task.progress}%` }}
            />
          </div>
          <div className="flex justify-between mt-1 text-xs text-gray-500">
            <span>{task.progress.toFixed(1)}%</span>
            <span>
              {task.speed && `${task.speed}`}
              {task.speed && task.eta && " · "}
              {task.eta && `剩余 ${task.eta}`}
            </span>
          </div>
        </div>
      )}

      {/* 已完成显示文件大小 */}
      {task.status === "completed" && task.filesize && (
        <div className="mt-2 text-xs text-gray-500">
          大小: {task.filesize}
        </div>
      )}

      {/* 错误信息 */}
      {task.error && (
        <div className="mt-2 text-xs text-red-400 bg-red-900/20 rounded p-2">
          {task.error}
        </div>
      )}

      {/* 操作按钮 */}
      <div className="flex gap-2 mt-2.5">
        {task.status === "downloading" && (
          <button
            onClick={() => pauseTask(task.id)}
            className="text-xs px-2.5 py-1 rounded bg-gray-800 text-gray-400
                       hover:bg-gray-700 hover:text-white transition-colors"
          >
            暂停
          </button>
        )}
        {task.status === "paused" && (
          <button
            onClick={() => requeueTask(task.id)}
            className="text-xs px-2.5 py-1 rounded bg-gray-800 text-gray-400
                       hover:bg-gray-700 hover:text-white transition-colors"
          >
            继续
          </button>
        )}
        {task.status === "failed" && (
          <button
            onClick={() => requeueTask(task.id)}
            className="text-xs px-2.5 py-1 rounded bg-gray-800 text-gray-400
                       hover:bg-gray-700 hover:text-white transition-colors"
          >
            重试
          </button>
        )}
        {(task.status === "downloading" ||
          task.status === "queued" ||
          task.status === "paused") && (
          <button
            onClick={() => cancelTask(task.id)}
            className="text-xs px-2.5 py-1 rounded bg-gray-800 text-red-400
                       hover:bg-red-900/30 hover:text-red-300 transition-colors"
          >
            取消
          </button>
        )}
      </div>
    </div>
  );
}