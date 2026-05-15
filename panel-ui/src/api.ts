import { useState, useEffect } from "react";
import { API } from "./constants";

export function useApi<T>(path: string, interval: number) {
  const [data, setData] = useState<T | null>(null);

  useEffect(() => {
    const fetchData = async () => {
      try {
        const res = await fetch(`${API}${path}`);
        setData(await res.json());
      } catch {
        /* mhw-radar.exe not running */
      }
    };
    fetchData();
    const id = setInterval(fetchData, interval);
    return () => clearInterval(id);
  }, [path, interval]);

  return data;
}
