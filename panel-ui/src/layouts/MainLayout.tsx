import type { ReactNode } from "react";

export function MainLayout({ children }: { children: ReactNode }) {
  return (
    <div
      style={{
        display: "flex",
        width: "100vw",
        height: "100vh",
        fontFamily: "system-ui, -apple-system, sans-serif",
        borderRadius: 20,
        overflow: "hidden",
        backgroundImage: "url(/background.png)",
        backgroundSize: "100% 100%",
        backgroundPosition: "center",
        backgroundRepeat: "no-repeat",
        border: "none",
        outline: "none",
        boxShadow: "none",
      }}
    >
      {children}
    </div>
  );
}
