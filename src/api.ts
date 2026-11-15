import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppSettings,
  CollectorStatus,
  CredentialsStatus,
  CrossHouseRow,
  Group,
  Health,
  HistoryRange,
  House,
  ItemHistory,
  MarketRow,
  MarketView,
  ItemTooltip,
  RealmType,
  Region,
  SnapshotEvent,
  TooltipLookup,
} from "./types";
import { createTooltipLoader } from "./tooltipLoader";

export const getMarket = (house: House) => invoke<MarketView>("get_market", { house });
export const getItemRow = (house: House, itemId: number, variantKey: string) => invoke<MarketRow | null>("get_item_row", { house, itemId, variantKey });
export const getItemHistory = (house: House, itemId: number, variantKey: string, range: HistoryRange) =>
  invoke<ItemHistory>("get_item_history", { house, itemId, variantKey, range });
export const getCrossHouse = (itemId: number, variantKey: string) => invoke<CrossHouseRow[]>("get_cross_house", { itemId, variantKey });
/** Item tooltips: session memory, then the local database, then Blizzard (stored for next time). */
export const loadTooltip = createTooltipLoader({
  cached: (itemId) => invoke<TooltipLookup>("get_cached_item_tooltip", { itemId }),
  fetch: (itemId) => invoke<ItemTooltip | null>("fetch_item_tooltip", { itemId }),
});
export const getGroups = () => invoke<Group[]>("get_groups");
export const createGroup = (name: string) => invoke<Group>("create_group", { name });
export const renameGroup = (id: number, name: string) => invoke<void>("rename_group", { id, name });
export const deleteGroup = (id: number) => invoke<void>("delete_group", { id });
export const setGroupMember = (groupId: number, itemId: number, variantKey: string, member: boolean) =>
  invoke<void>("set_group_member", { groupId, itemId, variantKey, member });
export const getHealth = () => invoke<Health>("get_health");
export const getSettings = () => invoke<AppSettings>("get_settings");
export const saveSettings = (settings: AppSettings) => invoke<void>("save_settings", { settings });
export const getCredentialsStatus = () => invoke<CredentialsStatus>("get_credentials_status");
export const saveCredentials = (blizzardClientId: string, blizzardClientSecret: string, ahledgerKey: string | null) =>
  invoke<void>("save_credentials", { blizzardClientId, blizzardClientSecret, ahledgerKey });
export const completeSetup = (region: Region, realmType: RealmType, blizzardClientId: string | null, blizzardClientSecret: string | null, ahledgerKey: string | null) =>
  invoke<void>("complete_setup", { region, realmType, blizzardClientId, blizzardClientSecret, ahledgerKey });
export const getCollectorStatus = () => invoke<CollectorStatus>("get_collector_status");
export const runCollectionNow = () => invoke<void>("run_collection_now");

export const onSnapshot = (cb: (e: SnapshotEvent) => void): Promise<UnlistenFn> =>
  listen<SnapshotEvent>("collector://snapshot", (e) => cb(e.payload));
export const onStatus = (cb: (s: CollectorStatus) => void): Promise<UnlistenFn> =>
  listen<CollectorStatus>("collector://status", (e) => cb(e.payload));
