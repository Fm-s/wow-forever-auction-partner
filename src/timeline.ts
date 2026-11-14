/** Horizontal position (0–100 %) of time `t` within [from, to]. */
export function tickPosition(t: number, from: number, to: number): number {
  if (to <= from) return 100;
  return Math.min(100, Math.max(0, ((t - from) / (to - from)) * 100));
}
