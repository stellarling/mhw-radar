export function formatTime(ms: number): string {
  const total_cs = Math.floor(ms / 10);
  const minutes = Math.floor(total_cs / 6000);
  const seconds = Math.floor((total_cs / 100) % 60);
  const centis = total_cs % 100;
  return `${minutes}'${String(seconds).padStart(2, "0")}'${String(centis).padStart(2, "0")}`;
}
