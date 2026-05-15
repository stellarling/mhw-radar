import { forwardRef } from "react";
import type { LogEntry } from "../types";
import { LOG_COLORS, btnStyle } from "../constants";

export const LogSection = forwardRef<HTMLDivElement, {
  entries: LogEntry[];
  total: number;
  autoScroll: boolean;
  onAutoScrollChange: (v: boolean) => void;
  onClear: () => void;
  onExport: () => void;
}>(function LogSection({ entries, total, autoScroll, onAutoScrollChange, onClear, onExport }, logEndRef) {

  // auto-scroll is triggered from parent via effect watching entries+autoScroll
  // expose ref so App can scrollIntoView
  return (
    <div id="hunting-log" style={{ display: "flex", flexDirection: "column", padding: "16px 20px" }}>
      <div style={{ display: "flex", alignItems: "center", marginBottom: 10 }}>
        <h2 style={{ color: "#dcdcdc", fontSize: 16, margin: 0 }}>狩猎日志</h2>
        <span style={{ color: "#8c8c8c", fontSize: 12, marginLeft: 10 }}>{total} 条</span>
        <label style={{ marginLeft: "auto", color: "#8c8c8c", fontSize: 13, cursor: "pointer", userSelect: "none" }}>
          <input type="checkbox" checked={autoScroll} onChange={() => onAutoScrollChange(!autoScroll)} />
          {" "}自动滚动
        </label>
      </div>

      <div
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

      <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 10 }}>
        <button onClick={onClear} style={btnStyle}>清除日志</button>
        <button onClick={onExport} style={btnStyle}>导出日志</button>
      </div>
    </div>
  );
});
