import { useState, useRef } from "react";
import { useStore } from "@/store";
import { open } from "@tauri-apps/plugin-dialog";
import CookiesModal from "@/components/CookiesModal";

export default function UrlInput() {
  const [url, setUrl] = useState("");
  const [quality, setQuality] = useState<"1080" | "720">("1080");
  const [showResolved, setShowResolved] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const addTask = useStore((s) => s.addTask);
  const outputDir = useStore((s) => s.outputDir);
  const setOutputDir = useStore((s) => s.setOutputDir);
  const cookiesFrom = useStore((s) => s.cookiesFrom);
  const resolveUrls = useStore((s) => s.resolveUrls);
  const resolvedEntries = useStore((s) => s.resolvedEntries);
  const resolving = useStore((s) => s.resolving);
  const clearResolved = useStore((s) => s.clearResolved);

  const handlePickFolder = async () => {
    const selected = await open({ directory: true, multiple: false });
    if (selected) setOutputDir(selected as string);
  };

  const handlePaste = async () => {
    try {
      const text = await navigator.clipboard.readText();
      if (text) setUrl(text.trim());
    } catch { /* ignore */ }
  };

  const handleResolve = async () => {
    const lines = url.split("\n").map((l) => l.trim()).filter(Boolean);
    if (lines.length === 0) return;
    clearResolved();
    await resolveUrls(lines);
    setShowResolved(true);
  };

  const makeFormat = () =>
    quality === "1080"
      ? "bestvideo[height<=1080]+bestaudio/best[height<=1080]/best"
      : "bestvideo[height<=720]+bestaudio/best[height<=720]/best";

  const handleDownloadAll = () => {
    const format = makeFormat();
    for (const entry of resolvedEntries) {
      addTask(entry.url, entry.title, format, cookiesFrom);
    }
    setUrl("");
    clearResolved();
    setShowResolved(false);
  };

  const handleDownloadSingle = (entry: { title: string; url: string }) => {
    const format = makeFormat();
    addTask(entry.url, entry.title, format, cookiesFrom);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && e.ctrlKey) {
      e.preventDefault();
      handleResolve();
    }
  };

  const lines = url.split("\n").filter(Boolean).length;

  return (
    <div className="px-5 py-4 border-b border-gray-800 bg-gray-900/50">
      {/* 保存位置 */}
      <div className="flex items-center gap-2 mb-3">
        <span className="text-xs text-gray-500 shrink-0">保存位置:</span>
        <button
          onClick={handlePickFolder}
          className="flex-1 text-left text-xs bg-gray-800 border border-gray-700 rounded px-3 py-1.5
                     text-gray-400 hover:text-gray-200 hover:border-gray-600 transition-colors truncate"
          title={outputDir || "点击选择文件夹"}
        >
          {outputDir || "点击选择文件夹..."}
        </button>
      </div>

      {/* URL 输入 — textarea 支持多行 */}
      <div className="flex gap-2 mb-2">
        <div className="flex-1 relative">
          <textarea
            ref={textareaRef}
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="粘贴视频 / 播放列表 URL，每行一个 ..."
            rows={3}
            className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2.5 text-sm
                       placeholder-gray-500 focus:outline-none focus:border-red-500 focus:ring-1 focus:ring-red-500
                       transition-colors resize-none"
          />
          <button
            type="button"
            onClick={handlePaste}
            className="absolute right-2 top-2 text-xs text-gray-500
                       hover:text-gray-300 bg-gray-700 px-2 py-0.5 rounded transition-colors"
          >
            粘贴
          </button>
          {lines > 1 && (
            <span className="absolute right-2 bottom-2 text-[10px] text-gray-600">
              {lines} 行
            </span>
          )}
        </div>
      </div>

      {/* 操作栏 */}
      <div className="flex items-center gap-3">
        <span className="text-xs text-gray-500">画质:</span>
        <button
          onClick={() => setQuality("1080")}
          className={`text-xs px-3 py-1.5 rounded transition-colors ${quality === "1080" ? "bg-red-600 text-white" : "bg-gray-800 text-gray-400 hover:bg-gray-700"}`}
        >
          超清 1080P
        </button>
        <button
          onClick={() => setQuality("720")}
          className={`text-xs px-3 py-1.5 rounded transition-colors ${quality === "720" ? "bg-red-600 text-white" : "bg-gray-800 text-gray-400 hover:bg-gray-700"}`}
        >
          高清 720P
        </button>
        <CookiesModal />

        <button
          onClick={handleResolve}
          disabled={!url.trim() || resolving}
          className="px-4 py-2 bg-gray-700 hover:bg-gray-600 disabled:opacity-40
                     text-white text-sm rounded-lg transition-colors"
        >
          {resolving ? "解析中..." : "解析"}
        </button>

        {resolvedEntries.length > 0 && (
          <button
            onClick={handleDownloadAll}
            className="px-5 py-2 bg-red-600 hover:bg-red-500
                       text-white text-sm font-medium rounded-lg transition-colors"
          >
            下载全部 ({resolvedEntries.length})
          </button>
        )}

        {resolvedEntries.length === 0 && !resolving && (
          <button
            onClick={() => {
              const trimmed = url.trim();
              if (!trimmed) return;
              const format = makeFormat();
              addTask(trimmed, trimmed, format, cookiesFrom);
              setUrl("");
            }}
            disabled={!url.trim()}
            className="ml-auto px-5 py-2 bg-red-600 hover:bg-red-500 disabled:bg-gray-700 disabled:text-gray-500
                       text-white text-sm font-medium rounded-lg transition-colors"
          >
            下载
          </button>
        )}
      </div>

      {/* 解析结果列表 */}
      {showResolved && resolvedEntries.length > 0 && (
        <div className="mt-3 border border-gray-700 rounded-lg overflow-hidden max-h-64 overflow-y-auto">
          <div className="px-3 py-2 bg-gray-800 border-b border-gray-700 flex items-center justify-between">
            <span className="text-xs text-gray-400">
              解析结果 ({resolvedEntries.length} 个视频)
            </span>
            <button
              onClick={() => { setShowResolved(false); clearResolved(); }}
              className="text-xs text-gray-500 hover:text-gray-300"
            >
              关闭
            </button>
          </div>
          {resolvedEntries.map((entry, i) => (
            <div
              key={i}
              className="flex items-center gap-2 px-3 py-1.5 hover:bg-gray-800/50 border-b border-gray-800 last:border-0"
            >
              <span className="text-[10px] text-gray-600 w-5 text-right">{i + 1}</span>
              <span className="text-xs text-gray-300 truncate flex-1" title={entry.title}>
                {entry.title}
              </span>
              <button
                onClick={() => handleDownloadSingle(entry)}
                className="text-[10px] text-gray-500 hover:text-red-400 px-2 py-0.5 rounded border border-gray-700 hover:border-red-700 shrink-0"
              >
                下载
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
