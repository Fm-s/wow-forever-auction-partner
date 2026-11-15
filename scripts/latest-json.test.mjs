import { describe, expect, it } from "vitest";
import { buildLatestJson, updaterKeys } from "./latest-json.mjs";

describe("updaterKeys", () => {
  it("maps the universal mac bundle to both mac architectures", () => {
    expect(updaterKeys("Auctioneer_universal.app.tar.gz")).toEqual(["darwin-aarch64", "darwin-x86_64", "darwin-aarch64-app", "darwin-x86_64-app"]);
  });
  it("makes NSIS the default Windows installer and also keys it as nsis", () => {
    expect(updaterKeys("Auctioneer_0.2.0_x64-setup.exe")).toEqual(["windows-x86_64", "windows-x86_64-nsis"]);
    expect(updaterKeys("Auctioneer_0.2.0_arm64-setup.exe")).toEqual(["windows-aarch64", "windows-aarch64-nsis"]);
  });
  it("keys MSI installs separately so they update through MSI", () => {
    expect(updaterKeys("Auctioneer_0.2.0_x64_en-US.msi")).toEqual(["windows-x86_64-msi"]);
  });
  it("makes AppImage the default Linux target and keys deb/rpm by installer", () => {
    expect(updaterKeys("Auctioneer_0.2.0_amd64.AppImage")).toEqual(["linux-x86_64", "linux-x86_64-appimage"]);
    expect(updaterKeys("Auctioneer_0.2.0_aarch64.AppImage")).toEqual(["linux-aarch64", "linux-aarch64-appimage"]);
    expect(updaterKeys("Auctioneer_0.2.0_arm64.deb")).toEqual(["linux-aarch64-deb"]);
    expect(updaterKeys("Auctioneer-0.2.0-1.x86_64.rpm")).toEqual(["linux-x86_64-rpm"]);
  });
  it("ignores files the updater cannot install", () => {
    expect(updaterKeys("Auctioneer_0.2.0_universal.dmg")).toEqual([]);
    expect(updaterKeys("latest.json")).toEqual([]);
  });
});

describe("buildLatestJson", () => {
  const repo = "Fm-s/wow-forever-auction-partner";
  const assets = [
    { name: "Auctioneer_universal.app.tar.gz", signature: "SIGMAC" },
    { name: "Auctioneer_0.2.0_x64-setup.exe", signature: "SIGWIN" },
    { name: "Auctioneer_0.2.0_universal.dmg", signature: null },
  ];
  const json = buildLatestJson({ version: "0.2.0", notes: "- A", pubDate: "2026-11-15T00:00:00Z", repo, assets });

  it("carries version, notes and date", () => {
    expect(json).toMatchObject({ version: "0.2.0", notes: "- A", pub_date: "2026-11-15T00:00:00Z" });
  });
  it("points each platform at the tagged download with its signature", () => {
    expect(json.platforms["darwin-aarch64"]).toEqual({ signature: "SIGMAC", url: `https://github.com/${repo}/releases/download/v0.2.0/Auctioneer_universal.app.tar.gz` });
    expect(json.platforms["windows-x86_64"].signature).toBe("SIGWIN");
  });
  it("skips assets without a signature", () => {
    expect(Object.keys(json.platforms)).not.toContain(undefined);
    expect(Object.keys(json.platforms).sort()).toEqual(["darwin-aarch64", "darwin-aarch64-app", "darwin-x86_64", "darwin-x86_64-app", "windows-x86_64", "windows-x86_64-nsis"]);
  });
  it("throws when no platform could be built", () => {
    expect(() => buildLatestJson({ version: "0.2.0", notes: "", pubDate: "x", repo, assets: [] })).toThrow(/no signed updater/i);
  });
});
