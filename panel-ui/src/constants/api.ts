export const API = "http://127.0.0.1:17320";

export const GITHUB_REPO = "stellarling/mhw-radar";
export const GITHUB_API_LATEST = `https://api.github.com/repos/${GITHUB_REPO}/releases/latest`;
export const GITHUB_RELEASE_DOWNLOAD = (tag: string, file: string) =>
  `https://github.com/${GITHUB_REPO}/releases/download/${tag}/${file}`;
