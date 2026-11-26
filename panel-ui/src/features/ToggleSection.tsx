import type { Settings, BoolKey } from "../types";
import { TOGGLES } from "../constants";
import { ToggleSwitch } from "../ui/ToggleSwitch";

export function ToggleSection({
  settings,
  onToggle,
}: {
  settings: Settings | null;
  onToggle: (key: BoolKey, value: boolean) => void;
}) {
  return (
    <div className="border-0 border-b border-solid border-panel-border p-4">
      <div className="mb-2.5 flex flex-wrap items-center gap-x-3 gap-y-1">
        <h2 className="m-0 text-base font-semibold text-panel-text">显示开关</h2>
        <span className="text-xs text-panel-muted">隐藏/显示 · Ctrl+Shift+U</span>
      </div>
      <div className="grid grid-cols-2 gap-x-2 gap-y-0.5 md:grid-cols-3 md:gap-x-6">
        {TOGGLES.map(({ key, label }) => (
          <ToggleSwitch
            key={key}
            label={label}
            checked={settings?.[key] ?? false}
            onChange={(checked) => onToggle(key, checked)}
          />
        ))}
      </div>
    </div>
  );
}
