import type { Group, MarketRow } from "./types";

export interface MarketFilter {
  search: string;
  itemClass: string | null;
  quality: string | null;
  /** Member keys of the selected groups; null = no group filter. */
  groupKeys: Set<string> | null;
}

export function memberKey(itemId: number, variantKey: string): string {
  return `${itemId}:${variantKey}`;
}

/** Union of the selected groups' members. Null when nothing (still existing) is selected. */
export function groupUnion(groups: Group[], selected: number[]): Set<string> | null {
  const chosen = groups.filter((g) => selected.includes(g.id));
  if (chosen.length === 0) return null;
  return new Set(chosen.flatMap((g) => g.members.map((m) => memberKey(m.itemId, m.variantKey))));
}

/** Name search is case-insensitive; a numeric search also matches the item id exactly. */
export function filterRows(rows: MarketRow[], f: MarketFilter): MarketRow[] {
  const q = f.search.trim().toLowerCase();
  const asId = /^\d+$/.test(q) ? Number(q) : null;
  return rows.filter(
    (r) =>
      (!q || r.name.toLowerCase().includes(q) || r.itemId === asId) &&
      (!f.itemClass || r.itemClass === f.itemClass) &&
      (!f.quality || r.quality === f.quality) &&
      (!f.groupKeys || f.groupKeys.has(memberKey(r.itemId, r.variantKey))),
  );
}

export function filterOptions(rows: MarketRow[]): { classes: string[]; qualities: string[] } {
  const uniq = (xs: (string | null)[]) => [...new Set(xs.filter((x): x is string => !!x))].sort();
  return { classes: uniq(rows.map((r) => r.itemClass)), qualities: uniq(rows.map((r) => r.quality)) };
}
