import type { Settings, BoolKey } from "../types";
import { TOGGLES } from "../constants";

export function ToggleSection({
  settings,
  onToggle,
}: {
  settings: Settings | null;
  onToggle: (key: BoolKey, value: boolean) => void;
}) {
  return (
    <div id="basic-tools" style={{ padding: "16px 20px", borderBottom: "1px solid #331e12" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 10 }}>
        <h2 style={{ color: "#dcdcdc", fontSize: 16, margin: 0 }}>显示开关</h2>
        <span style={{ color: "#b0b0b0", fontSize: 12 }}>隐藏/显示: Ctrl+Shift+U</span>
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: "2px 24px" }}>
        {TOGGLES.map(({ key, label }) => (
          <ToggleRow
            key={key}
            label={label}
            checked={settings ? Boolean(settings[key]) : false}
            onChange={(v) => onToggle(key, v)}
          />
        ))}
      </div>
    </div>
  );
}

function ToggleRow({ label, checked, onChange }: { label: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <div style={{ display: "flex", alignItems: "center", height: 28 }}>
      <div onClick={() => onChange(!checked)} style={{ cursor: "pointer", width: 38, height: 20, borderRadius: 10, flexShrink: 0,
        background: checked ? "linear-gradient(#BFA76B, #6C552D)" : "#444444", position: "relative",
        transition: "background 0.1s", border: "1px solid #666",
      }}>
        <div style={{
          width: 16, height: 16, borderRadius: "50%", background: "#fff",
          position: "absolute", top: 1,
          left: checked ? 20 : 2, transition: "left 0.1s",
        }} />
      </div>
      <span style={{ color: "#dcdcdc", fontSize: 15, marginLeft: 10, userSelect: "none" }}>{label}</span>
    </div>
  );
}
