import type { Settings } from "../types";

export function OpacitySection({
  settings,
  onChange,
}: {
  settings: Settings | null;
  onChange: (patch: Partial<Settings>) => void;
}) {
  return (
    <div className="grid grid-cols-2 divide-x divide-solid divide-panel-border border-0 border-b border-solid border-panel-border py-3">
      <SliderCompact
        label="悬浮窗框体透明度"
        value={settings?.window_opacity ?? 0.7}
        onChange={(window_opacity) => onChange({ window_opacity })}
      />
      <SliderCompact
        label="悬浮窗文字透明度"
        value={settings?.text_opacity ?? 1}
        onChange={(text_opacity) => onChange({ text_opacity })}
      />
    </div>
  );
}

function SliderCompact({ label, value, onChange }: {
  label: string;
  value: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="flex min-w-0 flex-col gap-2 px-4 text-xs text-panel-text md:text-sm">
      {label}
      <span className="flex items-center gap-2.5">
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={value}
          onChange={(event) => onChange(event.currentTarget.valueAsNumber)}
          className="m-0 min-w-0 max-w-35 flex-1 accent-panel-accent"
        />
        <span className="w-8 shrink-0 text-right text-xs text-panel-muted tabular-nums">
          {Math.round(value * 100)}%
        </span>
      </span>
    </label>
  );
}
