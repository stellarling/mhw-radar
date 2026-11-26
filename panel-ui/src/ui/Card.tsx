import type { HTMLAttributes } from "react";

export function Card({ className = "", ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={`min-w-0 rounded-md border border-solid border-panel-border bg-black/15 p-4 ${className}`} {...props} />;
}
