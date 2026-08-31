import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { DownloadTask, DownloadProgress, HistoryRecord } from "@/types";

interface AppStore {
  tasks: DownloadTask[];
  history: HistoryRecord[];
  activeTab: "download" | "history";
  outputDir: string;
  cookiesFrom: string;

  initOutputDir: () => Promise<void>;
  setOutputDir: (dir: string) => void;
  setCookiesFrom: (browser: string) => void;
  addTask: (url: string, title: string, formatId: string, cookiesFrom: string) => Promise<void>;
  refreshTasks: () => Promise<void>;
  pauseTask: (id: string) => Promise<void>;
  cancelTask: (id: string) => Promise<void>;
  requeueTask: (id: string) => Promise<void>;
  refreshHistory: (query?: string) => Promise<void>;
  deleteHistory: (id: string) => Promise<void>;
  setActiveTab: (tab: "download" | "history") => void;
}

export const useStore = create<AppStore>((set, get) => ({
  tasks: [],
  history: [],
  activeTab: "download",
  outputDir: "",
  cookiesFrom: "",

  initOutputDir: async () => {
    try {
      const dir = await invoke<string>("get_default_output_dir");
      set({ outputDir: dir });
    } catch {
      // 使用默认值
    }
  },

  setOutputDir: (dir: string) => set({ outputDir: dir }),

  setCookiesFrom: (browser: string) => set({ cookiesFrom: browser }),

  addTask: async (url: string, title: string, formatId: string, cookiesFrom: string) => {
    const outputDir = get().outputDir;
    await invoke("add_task", { url, title, formatId, outputDir, cookiesFrom: cookiesFrom || null });
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
}));

export async function initProgressListener() {
  await listen<DownloadProgress>("download-progress", (event) => {
    const { taskId, percent, speed, eta, totalBytes } = event.payload;
    useStore.setState((state) => ({
      tasks: state.tasks.map((t) =>
        t.id === taskId
          ? { ...t, progress: percent, speed, eta, filesize: totalBytes, status: "downloading" as const }
          : t
      ),
    }));
  });

  await listen<string>("task-updated", async () => {
    const tasks = await invoke<DownloadTask[]>("get_tasks");
    useStore.setState({ tasks });
  });
}