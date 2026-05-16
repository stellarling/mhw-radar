import type { ReactNode } from "react";

export function NavButton({
  label,
  icon,
  active,
  onClick,
}: {
  label: string;
  icon?: ReactNode;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <div
      onClick={onClick}
      style={{
        width: "100%",
        padding: "10px 14px",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        gap: 6,
        fontSize: 16,
        color: active ? "#fff" : "#a09080",
        background: active ? "linear-gradient(to right, #BFA76B, transparent)" : "transparent",
        cursor: "pointer",
        userSelect: "none",
        transition: "all 0.15s",
        lineHeight: 1.4,
      }}
    >
      {icon && <span style={{ display: "flex", flexShrink: 0 }}>{icon}</span>}
      {label}
    </div>
  );
}
