import type { MarketRow } from "./types";

export interface MarketFilter {
  search: string;
  itemClass: string | null;
  quality: string | null;
}

/** Name search is case-insensitive; a numeric search also matches the item id exactly. */
export function filterRows(rows: MarketRow[], f: MarketFilter): MarketRow[] {
  const q = f.search.trim().toLowerCase();
  const asId = /^\d+$/.test(q) ? Number(q) : null;
  return rows.filter(
    (r) =>
      (!q || r.name.toLowerCase().includes(q) || r.itemId === asId) &&
      (!f.itemClass || r.itemClass === f.itemClass) &&
      (!f.quality || r.quality === f.quality),
  );
}

export function filterOptions(rows: MarketRow[]): { classes: string[]; qualities: string[] } {
  const uniq = (xs: (string | null)[]) => [...new Set(xs.filter((x): x is string => !!x))].sort();
  return { classes: uniq(rows.map((r) => r.itemClass)), qualities: uniq(rows.map((r) => r.quality)) };
}
