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
      className="flex flex-col border-0 border-t border-solid border-panel-border p-4"
      style={style}
    >
      {children}
    </div>
  );
});
