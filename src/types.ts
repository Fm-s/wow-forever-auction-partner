// Mirrors the Rust structs serialized by the Tauri commands (camelCase).
export type House = "alliance" | "horde" | "blackwater";
export type Feed = "items" | "commodities";
export type HistoryRange = "24h" | "7d" | "30d" | "all";

export const HOUSES: House[] = ["alliance", "horde", "blackwater"];
export const HOUSE_LABEL: Record<House, string> = { alliance: "Alliance", horde: "Horde", blackwater: "Blackwater" };

export interface MarketRow {
  itemId: number;
  variantKey: string;
  name: string;
  quality: string | null;
  itemClass: string | null;
  itemSubclass: string | null;
  iconUrl: string | null;
  minPrice: number | null;
  median: number | null;
  quantity: number;
  median7d: number | null;
  estSoldPerDay: number | null;
  trendPct: number | null;
  ahlMin: number | null;
  ahlDeltaPct: number | null;
  feed: Feed;
}

export interface MarketView {
  house: House;
  asOf: number | null;
  partiallyStale: boolean;
  stale: boolean;
  rows: MarketRow[];
}

export interface CrossHouseRow {
  house: House;
  minPrice: number | null;
  median: number | null;
  quantity: number;
  cutBps: number;
  asOf: number;
}

export interface HistoryPoint {
  t: number;
  minPrice: number | null;
  median: number | null;
  quantity: number | null;
}

export interface ItemHistory {
  points: HistoryPoint[];
  /** est. units sold per day, normalised by that day's coverage; null when under 6 h covered */
  estSoldDaily: [string, number | null][];
  volatility7d: number | null;
  ahledger: [number, number | null][];
}

export interface FeedHealth {
  source: "blizzard" | "ahledger";
  house: House;
  feed: string;
  lastSuccess: number | null;
  lastErrorAt: number | null;
  lastError: string | null;
  lastModified: number | null;
  stale: boolean;
}

export interface TimelinePoint {
  house: House;
  feed: Feed;
  t: number;
  gap: boolean;
}

export interface Disagreement {
  house: House;
  itemId: number;
  variantKey: string;
  name: string;
  blizzardMin: number;
  ahledgerMin: number;
  deltaPct: number;
}

export interface Calibration {
  house: House;
  itemId: number;
  name: string;
  estSoldPerDay: number | null;
  ahlSalesPerDay: number;
}

export interface Health {
  feeds: FeedHealth[];
  timeline: TimelinePoint[];
  disagreements: Disagreement[];
  calibration: Calibration[];
}

export interface AppSettings {
  cadenceSecs: number;
  cutBpsAlliance: number;
  cutBpsHorde: number;
  cutBpsBlackwater: number;
  launchAtLogin: boolean;
  collectionPaused: boolean;
}

export interface CredentialsStatus {
  blizzardSet: boolean;
  ahledgerSet: boolean;
}

export type CollectorState = "starting" | "running" | "paused" | "error" | "no_credentials";

export interface CollectorStatus {
  state: CollectorState;
  lastCycleAt: number | null;
  lastNewSnapshotAt: number | null;
  message: string | null;
}

export interface SnapshotEvent {
  house: House;
  feed: Feed;
  snapshotId: number;
}

export interface ItemRef {
  itemId: number;
  variantKey: string;
}
