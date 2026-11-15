import type { ItemTooltip, TooltipLookup } from "./types";

export interface TooltipSource {
  /** Local database only. */
  cached: (itemId: number) => Promise<TooltipLookup>;
  /** Blizzard API; stores the result locally. */
  fetch: (itemId: number) => Promise<ItemTooltip | null>;
}

/** `onRemote` fires when the tooltip isn't stored locally and has to come from Blizzard. */
export type LoadTooltip = (itemId: number, onRemote: () => void) => Promise<ItemTooltip | null>;

interface Entry {
  promise: Promise<ItemTooltip | null>;
  remote: boolean;
  waiting: Set<() => void>;
}

/** Session cache in front of the local database: each item is looked up once per session. */
export function createTooltipLoader(source: TooltipSource): LoadTooltip {
  const entries = new Map<number, Entry>();
  return (itemId, onRemote) => {
    const existing = entries.get(itemId);
    if (existing) {
      if (existing.remote) onRemote();
      else existing.waiting.add(onRemote);
      return existing.promise;
    }
    const entry: Entry = { promise: Promise.resolve(null), remote: false, waiting: new Set([onRemote]) };
    entry.promise = source.cached(itemId).then((r) => {
      if (r.cached) return r.tooltip;
      entry.remote = true;
      entry.waiting.forEach((cb) => cb());
      entry.waiting.clear();
      return source.fetch(itemId);
    });
    entry.promise.catch(() => entries.delete(itemId));
    entries.set(itemId, entry);
    return entry.promise;
  };
}
