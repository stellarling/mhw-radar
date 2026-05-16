import type { PanelStatus } from "../types";
import { formatTime } from "../constants";

export function StatusBar({
  status,
  displayTime,
}: {
  status: PanelStatus | null;
  displayTime: number | null;
}) {
  return (
    <div style={{
      display: "grid",
      gridTemplateColumns: "1fr 1fr 1fr 1fr",
      alignItems: "center",
      padding: "0 20px",
      minHeight: 36,
      borderBottom: "1px solid #331e12",
      background: "transparent",
    }}>
      <div style={{ textAlign: "left" }}>
        <StatusDot connected={!!status?.connected} label={status?.connected ? "已连接" : "未连接"} />
        {status?.pid != null && (
          <span style={{ color: "#8c8c8c", fontSize: 12, marginLeft: 6 }}>
            PID {status.pid}
          </span>
        )}
      </div>
      <div style={{ textAlign: "left", color: "#dcdcdc", fontSize: 14, whiteSpace: "nowrap" }}>
        任务: {status?.quest_name || (status?.connected ? (status?.in_quest ? "任务中" : "无任务") : "未知")}
      </div>
      <div style={{ textAlign: "left", color: "#dcdcdc", fontSize: 14, whiteSpace: "nowrap" }}>
        目标: {status?.has_monster ? (status.monster_name || "未知") : "未知"}
      </div>
      <div style={{
        textAlign: "left",
        color: status?.in_quest && displayTime != null ? "#64c8ff" : "#8c8c8c",
        fontSize: 14,
        whiteSpace: "nowrap",
      }}>
        时间: {displayTime != null ? formatTime(displayTime) : "--"}
      </div>
    </div>
  );
}

function StatusDot({ connected, label }: { connected: boolean; label: string }) {
  return (
    <span style={{ color: connected ? "#4caf50" : "#888", fontSize: 14, whiteSpace: "nowrap" }}>
      <span style={{ marginRight: 4 }}>{connected ? "●" : "○"}</span>
      {label}
    </span>
  );
}
