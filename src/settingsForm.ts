import type { AppSettings, RealmType, Region } from "./types";

export interface SettingsForm {
  cadenceMin: string;
  cuts: { alliance: string; horde: string; blackwater: string };
  launchAtLogin: boolean;
  region: Region;
  realmType: RealmType;
}

/** "5" -> 500 basis points; null unless it's a number from 0 to 100. */
export function parseCutPercent(text: string): number | null {
  const t = text.trim();
  if (!t) return null;
  const v = Number(t);
  return Number.isFinite(v) && v >= 0 && v <= 100 ? Math.round(v * 100) : null;
}

/** Applies the form onto the latest stored settings. Pause is owned by its own toggle and the tray, so it's kept. */
export function applySettingsForm(fresh: AppSettings, form: SettingsForm): AppSettings {
  const minutes = Number(form.cadenceMin.trim());
  if (!Number.isFinite(minutes) || minutes < 5) throw new Error("Snapshot cadence must be at least 5 minutes.");
  const cut = (label: string, text: string) => {
    const v = parseCutPercent(text);
    if (v === null) throw new Error(`${label} cut must be a percentage from 0 to 100.`);
    return v;
  };
  return {
    ...fresh,
    cadenceSecs: Math.round(minutes * 60),
    cutBpsAlliance: cut("Alliance", form.cuts.alliance),
    cutBpsHorde: cut("Horde", form.cuts.horde),
    cutBpsBlackwater: cut("Blackwater", form.cuts.blackwater),
    launchAtLogin: form.launchAtLogin,
    region: form.region,
    realmType: form.realmType,
  };
}
