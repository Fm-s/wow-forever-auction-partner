/** Local "HH:MM" for a unix timestamp; "—" when unknown. */
export function hhmm(t: number | null | undefined): string {
  if (t === null || t === undefined) return "—";
  return new Date(t * 1000).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

/** Local "14 Nov, 17:17". */
export function dayTime(t: number | null | undefined): string {
  if (t === null || t === undefined) return "—";
  return new Date(t * 1000).toLocaleString([], { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" });
}

/** "4 min ago" style relative time. */
export function ago(t: number | null | undefined, now = Date.now() / 1000): string {
  if (t === null || t === undefined) return "never";
  const s = Math.max(0, Math.round(now - t));
  if (s < 90) return "just now";
  if (s < 5400) return `${Math.round(s / 60)} min ago`;
  if (s < 172800) return `${Math.round(s / 3600)} h ago`;
  return `${Math.round(s / 86400)} days ago`;
}
