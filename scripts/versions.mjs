// Version helpers shared by `pnpm release` and the release workflow.
// The app version lives in three files that must always agree with the git tag.
import { readFileSync } from "node:fs";
import { join } from "node:path";

const SEMVER = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+[0-9A-Za-z-.]+)?$/;

export const VERSION_FILES = {
  packageJson: "package.json",
  tauriConf: "src-tauri/tauri.conf.json",
  cargoToml: "src-tauri/Cargo.toml",
};
export const CARGO_LOCK = "src-tauri/Cargo.lock";
export const CRATE = "auctioneer";

export function isSemver(v) {
  return SEMVER.test(v);
}

export function isPrerelease(v) {
  const m = SEMVER.exec(v);
  return Boolean(m && m[4]);
}

/** Replace the top-level "version" of a JSON document, preserving its formatting. */
export function setJsonVersion(text, v) {
  // Top-level keys share the indent of the first key; nested ones are indented deeper.
  const indent = /^([ \t]+)"/m.exec(text)?.[1];
  if (!indent) throw new Error("expected a pretty-printed JSON object");
  const target = new RegExp(`^${indent}"version"\\s*:\\s*"[^"]*"`, "m");
  if (!target.test(text)) throw new Error("no top-level version field");
  return text.replace(target, (line) => line.replace(/"[^"]*"$/, `"${v}"`));
}

/** Replace `version` inside the [package] table of a Cargo.toml. */
export function setCargoTomlVersion(text, v) {
  return replaceInSection(text, /^\[package\]\s*$/m, v);
}

/** Replace the version of one crate's [[package]] entry in a Cargo.lock. */
export function setCargoLockVersion(text, crate, v) {
  const blocks = text.split(/(?=^\[\[package\]\]$)/m);
  let found = false;
  const out = blocks.map((b) => {
    if (!new RegExp(`^name = "${crate}"$`, "m").test(b)) return b;
    found = true;
    return b.replace(/^version = "[^"]*"$/m, `version = "${v}"`);
  });
  if (!found) throw new Error(`crate ${crate} not in Cargo.lock`);
  return out.join("");
}

function replaceInSection(text, header, v) {
  const start = text.search(header);
  if (start < 0) throw new Error("section not found");
  const rest = text.slice(start + 1);
  const next = rest.search(/^\[/m);
  const end = next < 0 ? text.length : start + 1 + next;
  const section = text.slice(start, end);
  if (!/^version\s*=\s*"[^"]*"/m.test(section)) throw new Error("no version in section");
  return text.slice(0, start) + section.replace(/^version\s*=\s*"[^"]*"/m, `version = "${v}"`) + text.slice(end);
}

export function readVersions(root) {
  const read = (f) => readFileSync(join(root, f), "utf8");
  const cargo = read(VERSION_FILES.cargoToml);
  const pkgSection = cargo.slice(cargo.search(/^\[package\]/m));
  return {
    packageJson: JSON.parse(read(VERSION_FILES.packageJson)).version,
    tauriConf: JSON.parse(read(VERSION_FILES.tauriConf)).version,
    cargoToml: /^version\s*=\s*"([^"]*)"/m.exec(pkgSection)?.[1],
  };
}

export function assertVersionsMatch(versions, expected) {
  const wrong = Object.entries(versions)
    .filter(([, v]) => v !== expected)
    .map(([k, v]) => `  ${VERSION_FILES[k]} is ${v ?? "missing"}`);
  if (wrong.length) throw new Error(`Version mismatch, expected ${expected}:\n${wrong.join("\n")}`);
}
