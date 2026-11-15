import { expect, test } from "vitest";
import { applySettingsForm, parseCutPercent } from "./settingsForm";
import type { AppSettings } from "./types";

const fresh: AppSettings = { cadenceSecs: 3600, cutBpsAlliance: 500, cutBpsHorde: 500, cutBpsBlackwater: 1500, launchAtLogin: true, collectionPaused: true, region: "us", realmType: "pvp", setupDone: true };

test("form fields override, but pause state comes from the latest stored settings", () => {
  const next = applySettingsForm(fresh, { cadenceMin: "30", cuts: { alliance: "4", horde: "5.5", blackwater: "15" }, launchAtLogin: false, region: "us", realmType: "pvp" });
  expect(next).toEqual({ ...fresh, cadenceSecs: 1800, cutBpsAlliance: 400, cutBpsHorde: 550, launchAtLogin: false, collectionPaused: true });
});

test("invalid values are rejected with a readable message", () => {
  const form = { cadenceMin: "60", cuts: { alliance: "5", horde: "5", blackwater: "15" }, launchAtLogin: true, region: "us" as const, realmType: "pvp" as const };
  expect(() => applySettingsForm(fresh, { ...form, cadenceMin: "2" })).toThrow(/at least 5 minutes/);
  expect(() => applySettingsForm(fresh, { ...form, cuts: { ...form.cuts, horde: "abc" } })).toThrow(/Horde cut/);
  expect(() => applySettingsForm(fresh, { ...form, cuts: { ...form.cuts, alliance: "120" } })).toThrow(/Alliance cut/);
});

test("parseCutPercent", () => {
  expect(parseCutPercent("5")).toBe(500);
  expect(parseCutPercent("-1")).toBeNull();
  expect(parseCutPercent("")).toBeNull();
});

test("the realm comes from the form; setup stays done", () => {
  const next = applySettingsForm(fresh, { cadenceMin: "60", cuts: { alliance: "5", horde: "5", blackwater: "15" }, launchAtLogin: true, region: "eu", realmType: "rppvp" });
  expect([next.region, next.realmType, next.setupDone]).toEqual(["eu", "rppvp", true]);
});
