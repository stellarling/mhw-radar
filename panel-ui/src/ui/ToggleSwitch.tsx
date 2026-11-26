export function ToggleSwitch({ label, checked, onChange }: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label className="flex min-h-7 cursor-pointer items-center gap-2 text-xs text-panel-text select-none md:text-sm">
      <input
        type="checkbox"
        role="switch"
        checked={checked}
        onChange={(event) => onChange(event.currentTarget.checked)}
        className="peer sr-only"
      />
      <span
        aria-hidden="true"
        className="relative h-5 w-9.5 shrink-0 rounded-full bg-neutral-700 ring-1 ring-neutral-500 ring-inset after:absolute after:top-0.5 after:left-0.5 after:size-4 after:rounded-full after:bg-white after:transition-transform peer-checked:bg-panel-accent peer-checked:after:translate-x-4.5 peer-focus-visible:ring-2 peer-focus-visible:ring-panel-gold"
      />
      {label}
    </label>
  );
}
