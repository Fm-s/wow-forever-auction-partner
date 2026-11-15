import type { RealmType, Region } from "./types";

export interface RealmChoice {
  region: Region;
  realmType: RealmType;
}

export const REGIONS: Region[] = ["us", "eu"];
export const REGION_LABEL: Record<Region, string> = { us: "US", eu: "EU" };
export const REALM_TYPES: RealmType[] = ["pvp", "normal", "rp", "rppvp"];
export const REALM_TYPE_LABEL: Record<RealmType, string> = { pvp: "PvP", normal: "Normal", rp: "Roleplay", rppvp: "Roleplay PvP" };

export function realmLabel(r: RealmChoice): string {
  return `${REALM_TYPE_LABEL[r.realmType]} · ${REGION_LABEL[r.region]}`;
}

/** What saving a realm change does; null when the realm is unchanged. */
export function realmSwitchNote(from: RealmChoice, to: RealmChoice): string | null {
  if (from.region === to.region && from.realmType === to.realmType) return null;
  return `Collection switches to ${realmLabel(to)} when you save. ${realmLabel(from)} history is kept; switch back anytime.`;
}
