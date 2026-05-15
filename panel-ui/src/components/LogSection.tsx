import { forwardRef, useState, useRef, useCallback, useEffect } from "react";
import type { LogEntry } from "../types";
import { LOG_COLORS, btnStyle } from "../constants";

const arrowBtnStyle: React.CSSProperties = {
  width: 22,
  height: 22,
  borderRadius: "50%",
  background: "#2a1a10",
  color: "#dcdcdc",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  cursor: "pointer",
  fontSize: 11,
  userSelect: "none",
  lineHeight: 1,
  flexShrink: 0,
  border: "none",
  fontFamily: "inherit",
  padding: 0,
};

function ToggleSwitch({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 8, cursor: "pointer", userSelect: "none" }}
      onClick={() => onChange(!checked)}
    >
      <div style={{
        width: 38, height: 20, borderRadius: 10, flexShrink: 0,
        background: checked ? "linear-gradient(#BFA76B, #6C552D)" : "#444444",
        position: "relative",
        border: "1px solid #666",
        transition: "background 0.1s",
      }}>
        <div style={{
          width: 16, height: 16, borderRadius: "50%", background: "#fff",
          position: "absolute", top: 1,
          left: checked ? 20 : 2, transition: "left 0.1s",
        }} />
      </div>
      <span style={{ color: "#b0b0b0", fontSize: 13 }}>{label}</span>
    </div>
  );
}

export const LogSection = forwardRef<HTMLDivElement, {
  entries: LogEntry[];
  totalRounds: number;
  currentRound: number;
  autoScroll: boolean;
  onAutoScrollChange: (v: boolean) => void;
  onPageChange: (page: number) => void;
  onClear: () => void;
  onExportCurrent: () => void;
  onExportAll: () => void;
}>(function LogSection(
  { entries, totalRounds, currentRound, autoScroll, onAutoScrollChange, onPageChange, onClear, onExportCurrent, onExportAll },
  logEndRef
) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [pageInput, setPageInput] = useState(String(currentRound + 1));
  const pageInputRef = useRef<HTMLInputElement>(null);
  const prevRoundRef = useRef(currentRound);

  // sync pageInput when currentRound changes externally
  useEffect(() => {
    setPageInput(String(currentRound + 1));
  }, [currentRound]);

  // 切换页时滚动到顶部
  useEffect(() => {
    if (prevRoundRef.current !== currentRound && containerRef.current) {
      containerRef.current.scrollTo({ top: 0, behavior: "auto" });
      prevRoundRef.current = currentRound;
    }
  }, [currentRound]);

  const handlePageSubmit = useCallback(() => {
    const p = parseInt(pageInput, 10);
    if (!isNaN(p) && p >= 1 && p <= totalRounds) {
      onPageChange(p - 1);
    } else {
      setPageInput(String(currentRound + 1));
    }
  }, [pageInput, totalRounds, currentRound, onPageChange]);

  return (
    <div id="hunting-log" style={{ display: "flex", flexDirection: "column", padding: "16px 20px" }}>
      {/* ── 标题行 ── */}
      <div style={{ display: "flex", alignItems: "center", marginBottom: 10, flexWrap: "wrap", gap: 6 }}>
        <h2 style={{ color: "#dcdcdc", fontSize: 16, margin: 0 }}>狩猎日志</h2>
        <span style={{ color: "#b0b0b0", fontSize: 12, marginLeft: 4 }}>
          {entries.length} 条 / {totalRounds} 轮
        </span>

        {/* 分页导航（跟随标题） */}
        <div style={{ display: "flex", alignItems: "center", gap: 3, marginLeft: 8 }}>
          <button
            style={arrowBtnStyle}
            disabled={currentRound <= 0}
            onClick={() => onPageChange(currentRound - 1)}
            title="上一页"
          >◀</button>

          <div style={{
            display: "flex", alignItems: "center", gap: 2,
            color: "#b0b0b0", fontSize: 13, userSelect: "none",
          }}>
            <input
              ref={pageInputRef}
              value={pageInput}
              onChange={(e) => setPageInput(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter") handlePageSubmit(); }}
              onBlur={handlePageSubmit}
              style={{
                width: 32, textAlign: "center",
                background: "#1a0f08", border: "1px solid #443322", borderRadius: 3,
                color: "#dcdcdc", fontSize: 13, padding: "2px 0", outline: "none",
              }}
            />
            <span style={{ color: "#b0b0b0" }}>/ {totalRounds}</span>
          </div>

          <button
            style={arrowBtnStyle}
            disabled={currentRound >= totalRounds - 1}
            onClick={() => onPageChange(currentRound + 1)}
            title="下一页"
          >▶</button>
        </div>

        {/* 自动滚动（右顶格） */}
        <div style={{ marginLeft: "auto" }}>
          <ToggleSwitch checked={autoScroll} onChange={onAutoScrollChange} label="自动滚动" />
        </div>
      </div>

      {/* ── 日志容器 ── */}
      <div
        ref={containerRef}
        className="log-container"
        style={{
          flex: "none",
          height: 360,
          overflowY: "auto",
          fontSize: 14,
          fontFamily: "Cascadia Code, JetBrains Mono, Consolas, monospace",
          lineHeight: 1.7,
          border: "1px solid #331e12",
          borderRadius: 4,
          padding: 10,
          background: "rgba(0,0,0,0.25)",
        }}
        onWheel={() => onAutoScrollChange(false)}
      >
        {entries.length === 0 ? (
          <div style={{ color: "#555", fontStyle: "italic", marginTop: 20, textAlign: "center" }}>
            等待游戏数据...
          </div>
        ) : (
          entries.map((entry, i) => (
            <div key={i} style={{ color: LOG_COLORS[entry.level] ?? "#dcdcdc" }}>
              <span style={{ color: "#8c8c8c" }}>{entry.timestamp}</span>{" "}
              <span>{entry.message}</span>
            </div>
          ))
        )}
        <div ref={logEndRef} />
      </div>

      {/* ── 底部按钮行 ── */}
      <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 10 }}>
        <button onClick={onClear} style={btnStyle}>清除日志</button>
        <button onClick={onExportAll} style={btnStyle}>导出全部</button>
        <button onClick={onExportCurrent} style={{
          ...btnStyle,
          background: "#4a2a15",
          color: "#f0d8b0",
        }}>导出本页</button>
      </div>
    </div>
  );
});
