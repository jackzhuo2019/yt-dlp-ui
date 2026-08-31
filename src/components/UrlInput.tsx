import { useState, useRef } from "react";
import { useStore } from "@/store";
import { open } from "@tauri-apps/plugin-dialog";

export default function UrlInput() {
  const [url, setUrl] = useState("");
  const [batchMode, setBatchMode] = useState(false);
  const [batchUrls, setBatchUrls] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const fetchFormats = useStore((s) => s.fetchFormats);
  const fetchingUrl = useStore((s) => s.fetchingUrl);
  const formatError = useStore((s) => s.formatError);
  const outputDir = useStore((s) => s.outputDir);
  const setOutputDir = useStore((s) => s.setOutputDir);

  const handlePickFolder = async () => {
    const selected = await open({ directory: true, multiple: false });
    if (selected) {
      setOutputDir(selected as string);
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = url.trim();
    if (!trimmed) return;
    fetchFormats(trimmed);
  };

  const handlePaste = async () => {
    try {
      const text = await navigator.clipboard.readText();
      if (text) {
        setUrl(text.trim());
      }
    } catch {
      // 剪贴板读取失败，忽略
    }
  };

  const handleBatchSubmit = () => {
    const urls = batchUrls
      .split("\n")
      .map((u) => u.trim())
      .filter(Boolean);
    if (urls.length === 0) return;
    fetchFormats(urls[0]);
    if (urls.length > 1) {
      setBatchUrls(urls.slice(1).join("\n"));
    }
  };

  return (
    <div className="px-5 py-4 border-b border-gray-800 bg-gray-900/50">
      {/* 保存位置 */}
      <div className="flex items-center gap-2 mb-3">
        <span className="text-xs text-gray-500 shrink-0">保存位置:</span>
        <button
          onClick={handlePickFolder}
          className="flex-1 text-left text-xs bg-gray-800 border border-gray-700 rounded px-3 py-1.5
                     text-gray-400 hover:text-gray-200 hover:border-gray-600 transition-colors
                     truncate"
          title={outputDir || "点击选择文件夹"}
        >
          {outputDir || "点击选择文件夹..."}
        </button>
      </div>

      <div className="flex items-center gap-2 mb-2">
        <button
          onClick={() => setBatchMode(false)}
          className={`text-xs px-2.5 py-1 rounded ${
            !batchMode
              ? "bg-red-600 text-white"
              : "bg-gray-800 text-gray-400 hover:text-white"
          }`}
        >
          单个下载
        </button>
        <button
          onClick={() => setBatchMode(true)}
          className={`text-xs px-2.5 py-1 rounded ${
            batchMode
              ? "bg-red-600 text-white"
              : "bg-gray-800 text-gray-400 hover:text-white"
          }`}
        >
          批量下载
        </button>
      </div>

      {!batchMode ? (
        <form onSubmit={handleSubmit} className="flex gap-2">
          <div className="flex-1 relative">
            <input
              ref={inputRef}
              type="text"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder="粘贴视频 / 播放列表 URL ..."
              className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2.5 text-sm
                         placeholder-gray-500 focus:outline-none focus:border-red-500 focus:ring-1 focus:ring-red-500
                         transition-colors"
            />
            <button
              type="button"
              onClick={handlePaste}
              className="absolute right-2 top-1/2 -translate-y-1/2 text-xs text-gray-500
                         hover:text-gray-300 bg-gray-700 px-2 py-0.5 rounded transition-colors"
            >
              粘贴
            </button>
          </div>
          <button
            type="submit"
            disabled={fetchingUrl === url || !url.trim()}
            className="px-5 py-2.5 bg-red-600 hover:bg-red-500 disabled:bg-gray-700 disabled:text-gray-500
                       text-white text-sm font-medium rounded-lg transition-colors shrink-0"
          >
            {fetchingUrl === url ? "获取中..." : "获取格式"}
          </button>
        </form>
      ) : (
        <div className="flex flex-col gap-2">
          <textarea
            value={batchUrls}
            onChange={(e) => setBatchUrls(e.target.value)}
            placeholder={"每行一个 URL，支持批量粘贴...\n\nhttps://www.youtube.com/watch?v=xxx\nhttps://www.youtube.com/watch?v=yyy"}
            rows={4}
            className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2.5 text-sm
                       placeholder-gray-500 focus:outline-none focus:border-red-500 focus:ring-1 focus:ring-red-500
                       transition-colors resize-none"
          />
          <div className="flex justify-end">
            <button
              onClick={handleBatchSubmit}
              disabled={!batchUrls.trim()}
              className="px-5 py-2 bg-red-600 hover:bg-red-500 disabled:bg-gray-700 disabled:text-gray-500
                         text-white text-sm font-medium rounded-lg transition-colors"
            >
              获取格式
            </button>
          </div>
        </div>
      )}

      {formatError && (
        <div className="mt-2 p-2.5 bg-red-900/30 border border-red-800 rounded-lg text-sm text-red-400">
          {formatError}
        </div>
      )}
    </div>
  );
}