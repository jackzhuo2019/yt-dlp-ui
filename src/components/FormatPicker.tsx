import { useState } from "react";
import { useStore } from "@/store";
import type { Format } from "@/types";

export default function FormatPicker() {
  const formatResult = useStore((s) => s.formatResult);
  const fetchingUrl = useStore((s) => s.fetchingUrl);
  const addTask = useStore((s) => s.addTask);
  const setFormatResult = useStore((s) => s.setFormatResult);

  const [selectedFormat, setSelectedFormat] = useState<string>("");
  const [filter, setFilter] = useState("");

  if (!formatResult) return null;

  const { title, formats } = formatResult;

  // 过滤：只看视频+音频合并格式，去重分辨率
  const seenRes = new Set<string>();
  const filtered = formats.filter((f) => {
    // 跳过纯音频
    if (f.vcodec === "none" && f.acodec !== "none") return false;
    // 跳过没有分辨率信息的
    if (!f.resolution || f.resolution === "audio only") return false;

    const key = `${f.resolution}_${f.ext}`;
    if (seenRes.has(key)) return false;
    seenRes.add(key);

    if (filter) {
      return (
        f.resolution.toLowerCase().includes(filter.toLowerCase()) ||
        f.ext.toLowerCase().includes(filter.toLowerCase()) ||
        f.formatNote.toLowerCase().includes(filter.toLowerCase())
      );
    }
    return true;
  });

  // 智能选择最佳格式
  const bestFormat = filtered.find(
    (f) => f.resolution === "1920x1080" || f.resolution === "1280x720"
  );
  const defaultFormat = bestFormat?.formatId || filtered[0]?.formatId || "";

  const handleDownload = () => {
    const fmt = selectedFormat || defaultFormat;
    if (!fmt) return;
    addTask(fetchingUrl, title, fmt);
    setFormatResult(null);
    setSelectedFormat("");
  };

  const formatSize = (bytes: number | null) => {
    if (bytes === null) return "未知";
    if (bytes > 1_000_000_000) return `${(bytes / 1_000_000_000).toFixed(1)} GB`;
    if (bytes > 1_000_000) return `${(bytes / 1_000_000).toFixed(1)} MB`;
    return `${(bytes / 1_000).toFixed(0)} KB`;
  };

  return (
    <div className="px-5 py-3 border-b border-gray-800 bg-gray-900/30">
      <div className="flex items-center justify-between mb-3">
        <div className="flex-1 min-w-0">
          <h3 className="text-sm font-medium text-gray-300 truncate">
            {title}
          </h3>
          <p className="text-xs text-gray-500 mt-0.5">
            共 {filtered.length} 种格式可选
          </p>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <input
            type="text"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="筛选..."
            className="w-28 bg-gray-800 border border-gray-700 rounded px-2 py-1 text-xs
                       placeholder-gray-500 focus:outline-none focus:border-red-500"
          />
          <button
            onClick={handleDownload}
            className="px-4 py-2 bg-red-600 hover:bg-red-500 text-white text-sm font-medium
                       rounded-lg transition-colors"
          >
            开始下载
          </button>
          <button
            onClick={() => setFormatResult(null)}
            className="px-2 py-2 text-gray-500 hover:text-gray-300 transition-colors"
          >
            ✕
          </button>
        </div>
      </div>

      <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-2 max-h-48 overflow-y-auto">
        {filtered.map((f) => {
          const isSelected =
            (selectedFormat || defaultFormat) === f.formatId;
          return (
            <button
              key={f.formatId}
              onClick={() => setSelectedFormat(f.formatId)}
              className={`text-left p-2 rounded-lg border text-xs transition-all ${
                isSelected
                  ? "border-red-500 bg-red-900/20 text-white"
                  : "border-gray-700 bg-gray-800/50 text-gray-400 hover:border-gray-600"
              }`}
            >
              <div className="font-medium text-white">{f.resolution}</div>
              <div className="text-gray-500 mt-0.5">
                {f.ext} · {f.formatNote || f.vcodec.split(".")[0]}
              </div>
              <div className="text-gray-600 mt-0.5">
                {formatSize(f.filesize)}
              </div>
            </button>
          );
        })}
      </div>
    </div>
  );
}