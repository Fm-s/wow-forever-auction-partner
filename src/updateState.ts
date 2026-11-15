export type UpdatePhase = "idle" | "available" | "downloading" | "ready" | "error";

/** Integer download progress 0–100, or null when the server didn't announce a size. */
export function progressPercent(downloaded: number, total: number | null | undefined): number | null {
  if (!total || total <= 0) return null;
  return Math.min(100, Math.floor((downloaded / total) * 100));
}
