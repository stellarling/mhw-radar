import type { PanelStatus } from "../types";
import { formatTime } from "../constants";

export function StatusBar({
  status,
  displayTime,
  demo = false,
}: {
  status: PanelStatus | null;
  displayTime: number | null;
  demo?: boolean;
}) {
  return (
    <div className="grid min-h-9 grid-cols-2 items-center gap-x-3 gap-y-1 border-0 border-b border-solid border-panel-border px-4 py-2 text-xs text-panel-text md:grid-cols-4 md:text-sm">
      <div>
        <StatusDot connected={!!status?.connected} label={demo ? "示例数据" : status?.connected ? "已连接" : "未连接"} />
        {status?.pid != null && (
          <span style={{ color: "#8c8c8c", fontSize: 12, marginLeft: 6 }}>
            PID {status.pid}
          </span>
        )}
      </div>
      <div>
        任务: {status?.quest_name || (status?.connected ? (status?.in_quest ? "任务中" : "无任务") : "未知")}
      </div>
      <div>
        目标: {status?.has_monster ? (status.monster_name || "未知") : "未知"}
      </div>
      <div className={status?.in_quest && displayTime != null ? "text-sky-300" : "text-panel-muted"}>
        时间: {displayTime != null ? formatTime(displayTime) : "--"}
      </div>
    </div>
  );
}

function StatusDot({ connected, label }: { connected: boolean; label: string }) {
  return (
    <span className={connected ? "text-green-400" : "text-panel-muted"}>
      <span style={{ marginRight: 4 }}>{connected ? "●" : "○"}</span>
      {label}
    </span>
  );
}
