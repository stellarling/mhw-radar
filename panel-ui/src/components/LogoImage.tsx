import { useState, useEffect } from "react";
import { API } from "../constants";

export function LogoImage({ size, fill }: { size?: number; fill?: boolean }) {
  const [ts, setTs] = useState(Date.now());
  useEffect(() => {
    const id = setInterval(() => setTs(Date.now()), 30000);
    return () => clearInterval(id);
  }, []);

  return (
    <img
      src={`${API}/api/resources/logo?t=${ts}`}
      alt="MHW Radar"
      style={{
        width: fill ? "100%" : (size ?? 100),
        opacity: 0.9,
        display: "block",
      }}
    />
  );
}
