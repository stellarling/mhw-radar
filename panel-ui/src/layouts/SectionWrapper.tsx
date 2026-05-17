import { forwardRef, type ReactNode } from "react";

export const SectionWrapper = forwardRef<HTMLDivElement, {
  id: string;
  children: ReactNode;
  style?: React.CSSProperties;
}>(function SectionWrapper({ id, children, style }, ref) {
  return (
    <div
      ref={ref}
      id={id}
      style={{
        padding: "16px 16px",
        borderTop: "1px solid #331e12",
        display: "flex",
        flexDirection: "column",
        ...style,
      }}
    >
      {children}
    </div>
  );
});
