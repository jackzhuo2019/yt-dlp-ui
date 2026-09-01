import { useState, useRef, useEffect } from "react";
import { useStore } from "@/store";
import { open } from "@tauri-apps/plugin-dialog";

const BROWSERS = [
  { id: "chrome", label: "Chrome", icon: "🌐" },
  { id: "edge", label: "Edge", icon: "🔷" },
  { id: "firefox", label: "Firefox", icon: "🦊" },
  { id: "brave", label: "Brave", icon: "🦁" },
  { id: "opera", label: "Opera", icon: "🔴" },
  { id: "vivaldi", label: "Vivaldi", icon: "🟠" },
  { id: "chromium", label: "Chromium", icon: "🔵" },
];

export default function CookiesModal() {
  const [isOpen, setIsOpen] = useState(false);
  const [extracting, setExtracting] = useState<string | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  const cookiesFrom = useStore((s) => s.cookiesFrom);
  const setCookiesFrom = useStore((s) => s.setCookiesFrom);
  const extractCookies = useStore((s) => s.extractCookies);
  const cookiesError = useStore((s) => s.cookiesError);

  // 点击外部关闭
  useEffect(() => {
    if (!isOpen) return;
    const handler = (e: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [isOpen]);

  const handlePickFile = async () => {
    setIsOpen(false);
    const selected = await open({
      multiple: false,
      filters: [{ name: "Cookies", extensions: ["txt"] }],
    });
    if (selected) {
      setCookiesFrom(selected as string);
    }
  };

  const handleExtract = async (browser: string) => {
    setExtracting(browser);
    try {
      await extractCookies(browser);
      setIsOpen(false);
    } finally {
      setExtracting(null);
    }
  };

  const handleClear = () => {
    setCookiesFrom("");
    setIsOpen(false);
  };

  const hasCookies = cookiesFrom.length > 0;
  const cookiesFileName = hasCookies
    ? cookiesFrom.split(/[\\/]/).pop() || cookiesFrom
    : "";

  return (
    <div className="relative" ref={panelRef}>
      <button
        onClick={() => setIsOpen(!isOpen)}
        className={`text-xs px-2 py-1.5 rounded transition-colors border ${
          hasCookies
            ? "bg-green-900/40 border-green-700 text-green-300 hover:bg-green-900/60"
            : "bg-gray-800 border-gray-700 text-gray-400 hover:text-gray-200 hover:border-gray-600"
        }`}
        title={hasCookies ? cookiesFrom : "Cookies"}
      >
        {hasCookies ? `Cookies: ${cookiesFileName}` : "Cookies"}
      </button>

      {isOpen && (
        <div className="absolute top-full left-0 mt-2 w-64 bg-gray-850 border border-gray-700 rounded-lg shadow-xl z-50 overflow-hidden">
          {/* 头部 */}
          <div className="px-3 py-2 border-b border-gray-700 flex items-center justify-between">
            <span className="text-xs text-gray-400">Cookies 来源</span>
            {hasCookies && (
              <button
                onClick={handleClear}
                className="text-xs text-gray-500 hover:text-red-400 transition-colors"
              >
                清除
              </button>
            )}
          </div>

          {/* 选择文件 */}
          <button
            onClick={handlePickFile}
            className="w-full text-left px-3 py-2 text-xs text-gray-300 hover:bg-gray-700/50 transition-colors flex items-center gap-2"
          >
            <span className="text-gray-500">📁</span>
            选择 cookies.txt 文件
          </button>

          {/* 分隔 */}
          <div className="px-3 py-1.5">
            <span className="text-[10px] uppercase tracking-wider text-gray-600">
              从浏览器提取
            </span>
          </div>

          {/* 浏览器列表 */}
          {BROWSERS.map((b) => {
            const isBusy = extracting === b.id;
            return (
              <button
                key={b.id}
                onClick={() => handleExtract(b.id)}
                disabled={isBusy}
                className="w-full text-left px-3 py-2 text-xs text-gray-300 hover:bg-gray-700/50 transition-colors flex items-center gap-2 disabled:opacity-50"
              >
                <span>{b.icon}</span>
                <span className="flex-1">{b.label}</span>
                {isBusy && (
                  <span className="text-gray-500 animate-pulse">提取中...</span>
                )}
              </button>
            );
          })}

          {/* 错误提示 */}
          {cookiesError && (
            <div className="px-3 py-2 border-t border-gray-700 text-xs text-red-400">
              {cookiesError}
            </div>
          )}
        </div>
      )}
    </div>
  );
}