import { describe, expect, it } from "vitest";
import { assertVersionsMatch, isPrerelease, isSemver, setCargoLockVersion, setCargoTomlVersion, setJsonVersion } from "./versions.mjs";

describe("isSemver", () => {
  it("accepts release and prerelease versions", () => {
    expect(isSemver("1.2.3")).toBe(true);
    expect(isSemver("1.2.3-beta.1")).toBe(true);
  });
  it("rejects tags and partial versions", () => {
    expect(isSemver("v1.2.3")).toBe(false);
    expect(isSemver("1.2")).toBe(false);
    expect(isSemver("01.2.3")).toBe(false);
  });
});

describe("isPrerelease", () => {
  it("is true only with a prerelease suffix", () => {
    expect(isPrerelease("1.0.0-rc.1")).toBe(true);
    expect(isPrerelease("1.0.0")).toBe(false);
  });
});

describe("setJsonVersion", () => {
  it("changes only the top-level version and keeps formatting", () => {
    const text = '{\n  "name": "x",\n  "version": "0.1.0",\n  "plugins": {\n    "a": { "version": "9.9.9" }\n  }\n}\n';
    const out = setJsonVersion(text, "0.2.0");
    expect(JSON.parse(out).version).toBe("0.2.0");
    expect(JSON.parse(out).plugins.a.version).toBe("9.9.9");
    expect(out).toBe(text.replace('"version": "0.1.0"', '"version": "0.2.0"'));
  });
});

describe("setCargoTomlVersion", () => {
  it("changes the [package] version, not dependency versions", () => {
    const text = '[package]\nname = "auctioneer"\nversion = "0.1.0"\n\n[dependencies]\ntauri = { version = "2" }\nserde = "1"\n';
    const out = setCargoTomlVersion(text, "1.0.0");
    expect(out).toContain('version = "1.0.0"');
    expect(out).toContain('tauri = { version = "2" }');
    expect(out.match(/1\.0\.0/g)).toHaveLength(1);
  });
});

describe("setCargoLockVersion", () => {
  it("changes only the named crate's block", () => {
    const text = '[[package]]\nname = "atoi"\nversion = "0.1.0"\n\n[[package]]\nname = "auctioneer"\nversion = "0.1.0"\ndependencies = [\n "atoi",\n]\n';
    const out = setCargoLockVersion(text, "auctioneer", "0.3.0");
    expect(out).toContain('name = "atoi"\nversion = "0.1.0"');
    expect(out).toContain('name = "auctioneer"\nversion = "0.3.0"');
  });
});

describe("assertVersionsMatch", () => {
  it("passes when every file matches", () => {
    expect(() => assertVersionsMatch({ packageJson: "0.2.0", tauriConf: "0.2.0", cargoToml: "0.2.0" }, "0.2.0")).not.toThrow();
  });
  it("names each mismatching file", () => {
    expect(() => assertVersionsMatch({ packageJson: "0.1.0", tauriConf: "0.2.0", cargoToml: "0.1.0" }, "0.2.0")).toThrow(/package\.json.*0\.1\.0[\s\S]*Cargo\.toml/);
  });
});
