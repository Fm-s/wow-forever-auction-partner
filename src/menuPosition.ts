/** Top-left for a w×h menu opened at the cursor (x, y), kept inside a vw×vh window. */
export function clampMenu(x: number, y: number, w: number, h: number, vw: number, vh: number, edge = 8): { left: number; top: number } {
  const left = x + w > vw - edge ? x - w : x;
  const top = Math.min(y, vh - h - edge);
  return { left: Math.max(edge, left), top: Math.max(edge, top) };
}
