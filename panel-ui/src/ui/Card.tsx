import type { ReactNode, CSSProperties } from "react";

const cardStyle: CSSProperties = {
  padding: 16,
  borderRadius: 6,
  border: "1px solid #331e12",
  background: "rgba(0,0,0,0.15)",
};

export function Card({ children, style, ...rest }: {
  children: ReactNode;
  style?: CSSProperties;
} & Record<string, unknown>) {
  return (
    <div style={{ ...cardStyle, ...style }} {...rest}>
      {children}
    </div>
  );
}
