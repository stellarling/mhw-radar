import type { ReactNode } from "react";

export function MainLayout({ children }: { children: ReactNode }) {
  return (
    <div className="panel-frame flex h-dvh w-full flex-col overflow-hidden rounded-[20px] font-sans md:flex-row">
      {children}
    </div>
  );
}
