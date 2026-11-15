export function qualityClass(q: string | null | undefined): string {
  return q ? `q-${q.toLowerCase()}` : "q-common";
}
