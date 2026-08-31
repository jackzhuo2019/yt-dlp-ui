import { useState, useRef } from "react";
import { useStore } from "@/store";
import { open } from "@tauri-apps/plugin-dialog";

export default function UrlInput() {
  const [url, setUrl] = useState("");
  const [quality, setQuality] = useState<"1080" | "720">("1080");
  const inputRef = useRef<HTMLInputElement>(null);

  const addTask = useStore((s) => s.addTask);
  const outputDir = useStore((s) => s.outputDir);
  const setOutputDir = useStore((s) => s.setOutputDir);
  const cookiesFrom = useStore((s) => s.cookiesFrom);
  const setCookiesFrom = useStore((s) => s.setCookiesFrom);

  const handlePickCookies = async () => {
    const selected = await open({ multiple: false, filters: [{ name: "Cookies", extensions: ["txt"] }] });
    if (selected) {
      setCookiesFrom(selected as string);
    }
  };

  const handlePickFolder = async () => {
    const selected = await open({ directory: true, multiple: false });
    if (selected) {
      setOutputDir(selected as string);
    }
  };

  const handlePaste = async () => {
    try {
      const text = await navigator.clipboard.readText();
      if (text) {
        setUrl(text.trim());
      }
    } catch {
      // 忽略
    }
  };

  const handleDownload = () => {
    const trimmed = url.trim();
    if (!trimmed) return;
    const format = quality === "1080" ? "best[height<=1080]/best" : "best[height<=720]/best";
    addTask(trimmed, trimmed, format, cookiesFrom);
    setUrl("");
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") handleDownload();
  };

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

      {/* URL 输入 */}
      <div className="flex gap-2 mb-3">
        <div className="flex-1 relative">
          <input
            ref={inputRef}
            type="text"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            onKeyDown={handleKeyDown}
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
      </div>

      {/* 画质选择 + Cookies + 下载 */}
      <div className="flex items-center gap-3">
        <span className="text-xs text-gray-500">画质:</span>
        <button
          onClick={() => setQuality("1080")}
          className={`text-xs px-3 py-1.5 rounded transition-colors ${
            quality === "1080"
              ? "bg-red-600 text-white"
              : "bg-gray-800 text-gray-400 hover:text-white"
          }`}
        >
          超清 1080P
        </button>
        <button
          onClick={() => setQuality("720")}
          className={`text-xs px-3 py-1.5 rounded transition-colors ${
            quality === "720"
              ? "bg-red-600 text-white"
              : "bg-gray-800 text-gray-400 hover:text-white"
          }`}
        >
          高清 720P
        </button>
        <button
          onClick={handlePickCookies}
          className="text-xs px-2 py-1.5 bg-gray-800 border border-gray-700 rounded text-gray-400
                     hover:text-gray-200 hover:border-gray-600 transition-colors truncate max-w-[120px]"
          title={cookiesFrom || "选择 cookies.txt 文件"}
        >
          {cookiesFrom ? "已选 cookies" : "Cookies"}
        </button>
        <button
          onClick={handleDownload}
          disabled={!url.trim()}
          className="ml-auto px-5 py-2 bg-red-600 hover:bg-red-500 disabled:bg-gray-700 disabled:text-gray-500
                     text-white text-sm font-medium rounded-lg transition-colors"
        >
          下载
        </button>
      </div>
    </div>
  );
}