#!/usr/bin/env node
// Builds the Tauri updater manifest (latest.json) once, from every signed asset of a release.
// Done in the publish job instead of per build job so parallel builds can't overwrite each other's entries.
// CLI: node scripts/latest-json.mjs --version 0.2.0 --repo owner/name --notes notes.md --assets dir/ > latest.json
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { stripVersionHeading } from "./release-body.mjs";
import { assertSemver } from "./versions.mjs";

function arch(name) {
  if (/(aarch64|arm64)/i.test(name)) return "aarch64";
  if (/(x64|amd64|x86_64)/i.test(name)) return "x86_64";
  return null;
}

/**
 * Updater platform keys an asset serves. The bare `{os}-{arch}` key is the fallback used by
 * installs whose installer type has no specific key; `{os}-{arch}-{installer}` keys win when present.
 */
export function updaterKeys(name) {
  if (/\.app\.tar\.gz$/.test(name)) {
    const archs = /universal/i.test(name) ? ["aarch64", "x86_64"] : [arch(name)].filter(Boolean);
    return [...archs.map((a) => `darwin-${a}`), ...archs.map((a) => `darwin-${a}-app`)];
  }
  const a = arch(name);
  if (!a) return [];
  if (/-setup\.exe$/i.test(name)) return [`windows-${a}`, `windows-${a}-nsis`];
  if (/\.msi$/i.test(name)) return [`windows-${a}-msi`];
  if (/\.AppImage$/.test(name)) return [`linux-${a}`, `linux-${a}-appimage`];
  if (/\.deb$/i.test(name)) return [`linux-${a}-deb`];
  if (/\.rpm$/i.test(name)) return [`linux-${a}-rpm`];
  return [];
}

export function buildLatestJson({ version, notes, pubDate, repo, assets }) {
  assertSemver(version);
  const platforms = {};
  for (const { name, signature } of assets) {
    if (!signature) continue;
    const url = `https://github.com/${repo}/releases/download/v${version}/${encodeURIComponent(name)}`;
    for (const key of updaterKeys(name)) platforms[key] = { signature, url };
  }
  if (!Object.keys(platforms).length) throw new Error("No signed updater artifacts found (expected *.sig next to installers)");
  return { version, notes, pub_date: pubDate, platforms };
}

function cli(argv) {
  const opt = (k) => argv[argv.indexOf(`--${k}`) + 1];
  const dir = opt("assets");
  const assets = readdirSync(dir)
    .filter((n) => !n.endsWith(".sig"))
    .map((name) => {
      const sig = join(dir, `${name}.sig`);
      return { name, signature: existsSync(sig) ? readFileSync(sig, "utf8").trim() : null };
    });
  const notes = stripVersionHeading(readFileSync(opt("notes"), "utf8"));
  const json = buildLatestJson({ version: opt("version"), notes, pubDate: new Date().toISOString(), repo: opt("repo"), assets });
  process.stdout.write(`${JSON.stringify(json, null, 2)}\n`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) cli(process.argv.slice(2));
