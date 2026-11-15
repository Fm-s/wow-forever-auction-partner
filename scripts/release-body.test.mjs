import { describe, expect, it } from "vitest";
import { classifyAsset, renderReleaseBody, topSection } from "./release-body.mjs";

const sha = (c) => c.repeat(64);
const assets = [
  { name: "WoW.Forever.Auction.Partner_0.2.0_universal.dmg", sha256: sha("a"), size: 12_500_000 },
  { name: "WoW.Forever.Auction.Partner_universal.app.tar.gz", sha256: sha("b"), size: 12_000_000 },
  { name: "WoW.Forever.Auction.Partner_universal.app.tar.gz.sig", sha256: sha("c"), size: 400 },
  { name: "WoW.Forever.Auction.Partner_0.2.0_x64-setup.exe", sha256: sha("d"), size: 5_000_000 },
  { name: "WoW.Forever.Auction.Partner_0.2.0_arm64_en-US.msi", sha256: sha("e"), size: 6_000_000 },
  { name: "WoW.Forever.Auction.Partner_0.2.0_amd64.AppImage", sha256: sha("f"), size: 80_000_000 },
  { name: "WoW.Forever.Auction.Partner_0.2.0_arm64.deb", sha256: sha("1"), size: 7_000_000 },
  { name: "WoW.Forever.Auction.Partner-0.2.0-1.x86_64.rpm", sha256: sha("2"), size: 7_100_000 },
  { name: "latest.json", sha256: sha("3"), size: 900 },
];
const base = { version: "0.2.0", repo: "Fm-s/wow-forever-auction-partner", notes: "### 🚀 Features\n\n- Item groups", assets, prerelease: false };

describe("classifyAsset", () => {
  it("maps installers to platform, arch and kind", () => {
    expect(classifyAsset("WoW.Forever.Auction.Partner_0.2.0_universal.dmg")).toEqual({ platform: "macOS", arch: "Universal (Apple Silicon + Intel)", kind: "Disk image (.dmg)" });
    expect(classifyAsset("WoW.Forever.Auction.Partner_0.2.0_x64-setup.exe")).toMatchObject({ platform: "Windows", arch: "x64" });
    expect(classifyAsset("WoW.Forever.Auction.Partner_0.2.0_arm64_en-US.msi")).toMatchObject({ platform: "Windows", arch: "ARM64" });
    expect(classifyAsset("WoW.Forever.Auction.Partner_0.2.0_amd64.AppImage")).toMatchObject({ platform: "Linux", arch: "x64" });
    expect(classifyAsset("WoW.Forever.Auction.Partner_0.2.0_arm64.deb")).toMatchObject({ platform: "Linux", arch: "ARM64" });
    expect(classifyAsset("WoW.Forever.Auction.Partner-0.2.0-1.x86_64.rpm")).toMatchObject({ platform: "Linux", arch: "x64" });
    expect(classifyAsset("WoW.Forever.Auction.Partner-0.2.0-1.aarch64.rpm")).toMatchObject({ platform: "Linux", arch: "ARM64" });
  });
  it("returns null for updater and metadata files", () => {
    for (const n of ["WoW.Forever.Auction.Partner_universal.app.tar.gz", "x.sig", "latest.json", "SHA256SUMS.txt", "wow-forever-auction-partner-0.2.0.spdx.json"]) expect(classifyAsset(n)).toBeNull();
  });
});

describe("renderReleaseBody", () => {
  it("includes the notes verbatim", () => {
    expect(renderReleaseBody(base)).toContain("### 🚀 Features\n\n- Item groups");
  });
  it("drops git-cliff's own version heading from the notes", () => {
    const body = renderReleaseBody({ ...base, notes: "## [0.2.0](https://x/releases/tag/v0.2.0) — 2026-11-15\n\n### 🚀 Features\n\n- A" });
    expect(body).not.toContain("## [0.2.0]");
    expect(body).toContain("- A");
  });
  it("lists installers in the downloads table but not updater files", () => {
    const table = renderReleaseBody(base).split("## ⬇️ Downloads")[1].split("\n## ")[0];
    expect(table).toContain("WoW.Forever.Auction.Partner_0.2.0_universal.dmg");
    expect(table).toContain("WoW.Forever.Auction.Partner-0.2.0-1.x86_64.rpm");
    expect(table).not.toContain(".app.tar.gz");
    expect(table).not.toContain("latest.json");
    expect(table.indexOf("macOS")).toBeLessThan(table.indexOf("Windows"));
    expect(table.indexOf("Windows")).toBeLessThan(table.indexOf("Linux"));
  });
  it("orders x64 before ARM64 and the .exe installer before the MSI", () => {
    const body = renderReleaseBody({
      ...base,
      assets: [
        { name: "WoW.Forever.Auction.Partner_0.2.0_arm64-setup.exe", sha256: sha("4"), size: 1 },
        { name: "WoW.Forever.Auction.Partner_0.2.0_x64_en-US.msi", sha256: sha("5"), size: 1 },
        { name: "WoW.Forever.Auction.Partner_0.2.0_x64-setup.exe", sha256: sha("6"), size: 1 },
      ],
    });
    const order = ["x64-setup.exe", "x64_en-US.msi", "arm64-setup.exe"].map((n) => body.indexOf(n));
    expect(order).toEqual([...order].sort((a, b) => a - b));
  });

  it("rejects a missing or non-semver version", () => {
    for (const version of ["", undefined, "v0.2.0"]) expect(() => renderReleaseBody({ ...base, version })).toThrow(/version/i);
  });

  it("warns macOS users about the keychain prompt after updates", () => {
    expect(renderReleaseBody(base)).toContain("Always Allow");
  });

  it("lists every asset's checksum", () => {
    const body = renderReleaseBody(base);
    for (const a of assets) expect(body).toContain(`${a.sha256}  ${a.name}`);
  });
  it("url-encodes download links", () => {
    const body = renderReleaseBody({ ...base, assets: [{ name: "WoW Forever Auction Partner (beta).dmg", sha256: sha("9"), size: 1 }] });
    expect(body).toContain("/releases/download/v0.2.0/WoW%20Forever%20Auction%20Partner%20(beta).dmg");
  });
  it("warns on prereleases only", () => {
    expect(renderReleaseBody({ ...base, prerelease: true })).toContain("Pre-release");
    expect(renderReleaseBody(base)).not.toContain("Pre-release");
  });
  it("explains first launch of unsigned builds and verification", () => {
    const body = renderReleaseBody(base);
    expect(body).toContain("Open Anyway");
    expect(body).toContain("Run anyway");
    expect(body).toContain("shasum -a 256 -c SHA256SUMS.txt --ignore-missing");
    expect(body).toContain("gh attestation verify WoW.Forever.Auction.Partner_0.2.0_universal.dmg -R Fm-s/wow-forever-auction-partner");
  });
});

describe("topSection", () => {
  it("returns only the newest release section of a changelog", () => {
    const md = "## [0.3.0](u) — d\n\n### 🚀 Features\n\n- New\n\n## [0.2.0](u) — d\n\n### 🐛 Fixes\n\n- Old\n";
    expect(topSection(md)).toBe("## [0.3.0](u) — d\n\n### 🚀 Features\n\n- New");
  });
  it("returns the whole text when there is a single section", () => {
    expect(topSection("## [0.1.0](u)\n\n- A\n")).toBe("## [0.1.0](u)\n\n- A");
  });
});
