import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  DownloadTask,
  DownloadProgress,
  FormatResult,
  HistoryRecord,
} from "@/types";

interface AppStore {
  // 任务列表
  tasks: DownloadTask[];
  // 历史记录
  history: HistoryRecord[];
  // 当前正在查看格式的 URL
  fetchingUrl: string;
  formatResult: FormatResult | null;
  formatError: string | null;
  // 活跃标签页
  activeTab: "download" | "history";

  // 操作
  fetchFormats: (url: string) => Promise<void>;
  addTask: (url: string, title: string, formatId: string) => Promise<void>;
  refreshTasks: () => Promise<void>;
  pauseTask: (id: string) => Promise<void>;
  cancelTask: (id: string) => Promise<void>;
  requeueTask: (id: string) => Promise<void>;
  refreshHistory: (query?: string) => Promise<void>;
  deleteHistory: (id: string) => Promise<void>;
  setActiveTab: (tab: "download" | "history") => void;
  setFormatResult: (result: FormatResult | null) => void;
  setFormatError: (error: string | null) => void;
  setFetchingUrl: (url: string) => void;
}

export const useStore = create<AppStore>((set, get) => ({
  tasks: [],
  history: [],
  fetchingUrl: "",
  formatResult: null,
  formatError: null,
  activeTab: "download",

  fetchFormats: async (url: string) => {
    set({ fetchingUrl: url, formatError: null, formatResult: null });
    try {
      const result = await invoke<FormatResult>("fetch_formats", { url });
      set({ formatResult: result });
    } catch (e: any) {
      set({ formatError: String(e) });
    }
  },

  addTask: async (url: string, title: string, formatId: string) => {
    await invoke("add_task", { url, title, formatId });
    await get().refreshTasks();
  },

  refreshTasks: async () => {
    const tasks = await invoke<DownloadTask[]>("get_tasks");
    set({ tasks });
  },

  pauseTask: async (id: string) => {
    await invoke("pause_task", { taskId: id });
    await get().refreshTasks();
  },

  cancelTask: async (id: string) => {
    await invoke("cancel_task", { taskId: id });
    await get().refreshTasks();
  },

  requeueTask: async (id: string) => {
    await invoke("requeue_task", { taskId: id });
    await get().refreshTasks();
  },

  refreshHistory: async (query?: string) => {
    const history = await invoke<HistoryRecord[]>("get_history", {
      query: query || null,
    });
    set({ history });
  },

  deleteHistory: async (id: string) => {
    await invoke("delete_history", { id });
    await get().refreshHistory();
  },

  setActiveTab: (tab) => set({ activeTab: tab }),
  setFormatResult: (result) => set({ formatResult: result }),
  setFormatError: (error) => set({ formatError: error }),
  setFetchingUrl: (url) => set({ fetchingUrl: url }),
}));

// 监听下载进度事件
export async function initProgressListener() {
  await listen<DownloadProgress>("download-progress", (event) => {
    const { taskId, percent, speed, eta, totalBytes } = event.payload;
    useStore.setState((state) => ({
      tasks: state.tasks.map((t) =>
        t.id === taskId
          ? {
              ...t,
              progress: percent,
              speed,
              eta,
              filesize: totalBytes,
              status: "downloading" as const,
            }
          : t
      ),
    }));
  });

  // 监听任务状态更新
  await listen<string>("task-updated", async (event) => {
    // 刷新任务列表
    const tasks = await invoke<DownloadTask[]>("get_tasks");
    useStore.setState({ tasks });
  });
}