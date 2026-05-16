import type { Settings } from "../types";

export function OpacitySection({
  settings,
  onChange,
}: {
  settings: Settings | null;
  onChange: (patch: Partial<Settings>) => void;
}) {
  return (
    <div className="no-drag" style={{
      display: "flex",
      borderBottom: "1px solid #331e12",
      padding: "12px 20px",
    }}>
      <div style={{ flex: 1, paddingRight: 20, borderRight: "1px solid #331e12" }}>
        <div style={{ color: "#dcdcdc", fontSize: 14, marginBottom: 8 }}>悬浮窗框体-透明度</div>
        <SliderCompact
          value={settings?.window_opacity ?? 0.7}
          onChange={(v) => onChange({ window_opacity: v })}
        />
      </div>
      <div style={{ flex: 1, paddingLeft: 20 }}>
        <div style={{ color: "#dcdcdc", fontSize: 14, marginBottom: 8 }}>悬浮窗文本-透明度</div>
        <SliderCompact
          value={settings?.text_opacity ?? 1.0}
          onChange={(v) => onChange({ text_opacity: v })}
        />
      </div>
    </div>
  );
}

function SliderCompact({ value, onChange }: { value: number; onChange: (v: number) => void }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
      <input
        type="range"
        min="0"
        max="1"
        step="0.01"
        value={value}
        onChange={(e) => onChange(Math.round(parseFloat(e.target.value) * 100) / 100)}
        style={{ width: 140, accentColor: "#4688e6", height: 4 }}
      />
      <span style={{ color: "#8c8c8c", fontSize: 13, width: 32, textAlign: "right", flexShrink: 0 }}>
        {Math.round(value * 100)}%
      </span>
    </div>
  );
}
