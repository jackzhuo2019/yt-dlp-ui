import { useEffect } from "react";
import { useStore, initProgressListener } from "@/store";
import UrlInput from "@/components/UrlInput";
import DownloadQueue from "@/components/DownloadQueue";
import History from "@/components/History";

export default function App() {
  const activeTab = useStore((s) => s.activeTab);
  const setActiveTab = useStore((s) => s.setActiveTab);
  const initOutputDir = useStore((s) => s.initOutputDir);

  useEffect(() => {
    initProgressListener();
    initOutputDir();
  }, []);

  return (
    <div className="h-screen flex flex-col">
      {/* 标题栏 */}
      <header className="flex items-center justify-between px-5 py-3 border-b border-gray-800 bg-gray-900">
        <div className="flex items-center gap-3">
          <h1 className="text-lg font-bold tracking-tight">
            <span className="text-red-500">yt-dlp</span> UI
          </h1>
          <span className="text-xs text-gray-500 bg-gray-800 px-2 py-0.5 rounded">
            v0.1.0
          </span>
        </div>
        <nav className="flex gap-1 bg-gray-800 rounded-lg p-0.5">
          <button
            onClick={() => setActiveTab("download")}
            className={`px-4 py-1.5 text-sm rounded-md transition-colors ${
              activeTab === "download"
                ? "bg-gray-700 text-white"
                : "text-gray-400 hover:text-white"
            }`}
          >
            下载
          </button>
          <button
            onClick={() => setActiveTab("history")}
            className={`px-4 py-1.5 text-sm rounded-md transition-colors ${
              activeTab === "history"
                ? "bg-gray-700 text-white"
                : "text-gray-400 hover:text-white"
            }`}
          >
            历史记录
          </button>
        </nav>
      </header>

      {/* 主内容 */}
      <main className="flex-1 overflow-hidden">
        {activeTab === "download" ? (
          <div className="h-full flex flex-col">
            <UrlInput />
            <DownloadQueue />
          </div>
        ) : (
          <History />
        )}
      </main>
    </div>
  );
}