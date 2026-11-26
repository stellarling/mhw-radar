import { useState, useEffect } from "react";
import { API } from "../constants";

export function LogoImage({ size, fill, src }: { src?: string; size?: number; fill?: boolean }) {
  const [ts, setTs] = useState(Date.now());
  useEffect(() => {
    if (src) return;
    const id = setInterval(() => setTs(Date.now()), 30000);
    return () => clearInterval(id);
  }, [src]);

  return (
    <img
      src={src ?? `${API}/api/resources/logo?t=${ts}`}
      alt="MHW Radar"
      style={{
        width: fill ? "100%" : (size ?? 100),
        opacity: 0.9,
        display: "block",
      }}
    />
  );
}
