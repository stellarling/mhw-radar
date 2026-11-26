import type { ReactNode } from "react";

export function NavButton({
  label,
  icon,
  active,
  onClick,
}: {
  label: string;
  icon: ReactNode;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-current={active ? "true" : undefined}
      onClick={onClick}
      className={`flex cursor-pointer items-center justify-center gap-1 rounded-none border-0 bg-transparent px-1.5 py-2 text-xs leading-tight transition-colors md:w-full md:gap-2 md:px-3.5 md:py-2.5 md:text-base ${
        active ? "bg-linear-to-r from-panel-accent/80 to-transparent text-white" : "text-panel-muted hover:bg-white/5 hover:text-panel-text"
      }`}
    >
      <span className="hidden shrink-0 md:flex">{icon}</span>
      {label}
    </button>
  );
}
