import type { ConnectionLogEntry, ConnectionEventType } from "../types";

const EVENT_LABELS: Record<ConnectionEventType, string> = {
  Waiting: "等待",
  Connected: "已连接",
  Disconnected: "已断开",
  Reconnected: "已重连",
  ReadError: "读取异常",
  Info: "信息",
};

const EVENT_COLORS: Record<ConnectionEventType, string> = {
  Waiting: "#8c8c8c",
  Connected: "#4caf50",
  Disconnected: "#e53935",
  Reconnected: "#ffa726",
  ReadError: "#ef5350",
  Info: "#64b5f6",
};

export function ConnectionDiagnostics({
  logs,
}: {
  logs: ConnectionLogEntry[];
}) {
  const latest = logs.length > 0 ? logs[logs.length - 1] : null;

  return (
    <div
      className="mx-4 my-3 overflow-hidden rounded-sm border border-solid border-panel-border bg-black/15"
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          padding: "4px 10px",
          userSelect: "none",
          minHeight: 28,
          gap: 8,
        }}
      >
        <span style={{ color: "#dcdcdc", fontSize: 14, fontWeight: 600, flexShrink: 0 }}>
          连接诊断
        </span>

        {latest ? (
          <span
            style={{
              color: EVENT_COLORS[latest.event_type] ?? "#dcdcdc",
              fontSize: 12,
              whiteSpace: "nowrap",
              overflow: "hidden",
              textOverflow: "ellipsis",
              minWidth: 0,
            }}
          >
            <span style={{ color: "#8c8c8c" }}>{latest.timestamp}</span>{" "}
            <span style={{ fontSize: 12, opacity: 0.8, marginRight: 2 }}>
              [{EVENT_LABELS[latest.event_type] ?? latest.event_type}]
            </span>
            <span>{latest.message}</span>
          </span>
        ) : (
          <span style={{ color: "#8c8c8c", fontSize: 12, fontStyle: "italic" }}>
            暂无连接事件
          </span>
        )}
      </div>
    </div>
  );
}
