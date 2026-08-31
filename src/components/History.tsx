import { useEffect, useState } from "react";
import { useStore } from "@/store";

export default function History() {
  const history = useStore((s) => s.history);
  const refreshHistory = useStore((s) => s.refreshHistory);
  const deleteHistory = useStore((s) => s.deleteHistory);
  const [search, setSearch] = useState("");

  useEffect(() => {
    refreshHistory();
  }, []);

  const handleSearch = () => {
    refreshHistory(search || undefined);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") handleSearch();
  };

  // 格式化日期
  const formatDate = (iso: string) => {
    try {
      const d = new Date(iso);
      return d.toLocaleString("zh-CN", {
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
      });
    } catch {
      return iso;
    }
  };

  return (
    <div className="h-full flex flex-col">
      {/* 搜索栏 */}
      <div className="px-5 py-3 border-b border-gray-800 flex gap-2">
        <input
          type="text"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="搜索标题或 URL..."
          className="flex-1 bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm
                     placeholder-gray-500 focus:outline-none focus:border-red-500
                     transition-colors"
        />
        <button
          onClick={handleSearch}
          className="px-4 py-2 bg-gray-800 hover:bg-gray-700 text-gray-300 text-sm
                     rounded-lg transition-colors"
        >
          搜索
        </button>
      </div>

      {/* 历史列表 */}
      <div className="flex-1 overflow-y-auto">
        {history.length === 0 ? (
          <div className="flex items-center justify-center h-full text-gray-600">
            <div className="text-center">
              <div className="text-4xl mb-3">📭</div>
              <p className="text-sm">暂无下载记录</p>
            </div>
          </div>
        ) : (
          <div className="divide-y divide-gray-800/50">
            {history.map((record) => (
              <div
                key={record.id}
                className="px-5 py-3 hover:bg-gray-900/50 transition-colors"
              >
                <div className="flex items-start justify-between gap-3">
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate">
                      {record.title}
                    </p>
                    <p className="text-xs text-gray-600 truncate mt-0.5">
                      {record.url}
                    </p>
                    <div className="flex items-center gap-3 mt-1.5 text-xs text-gray-500">
                      <span>
                        {record.resolution} · {record.ext}
                      </span>
                      {record.filesize && <span>{record.filesize}</span>}
                      <span>{formatDate(record.downloadedAt)}</span>
                    </div>
                  </div>
                  <button
                    onClick={() => deleteHistory(record.id)}
                    className="text-xs px-2 py-1 rounded text-gray-600 hover:text-red-400
                               hover:bg-red-900/20 transition-colors shrink-0"
                  >
                    删除
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}