export async function readApiError(res: Response): Promise<string> {
  try {
    const data = await res.json();
    if (typeof data?.error === "string" && data.error.trim()) {
      return data.error;
    }
  } catch {
    /* response is not JSON */
  }

  try {
    const text = await res.text();
    if (text.trim()) {
      return text;
    }
  } catch {
    /* ignore */
  }

  return `HTTP ${res.status}`;
}
