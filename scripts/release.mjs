#!/usr/bin/env node
// Usage: pnpm release <version> [--dry-run] [--allow-branch]
// Bumps every version file, regenerates CHANGELOG.md, commits and tags v<version>.
// Pushing the tag (git push origin main --follow-tags) triggers the release workflow.
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { CARGO_LOCK, CRATE, VERSION_FILES, isSemver, readVersions, setCargoLockVersion, setCargoTomlVersion, setJsonVersion } from "./versions.mjs";

const root = fileURLToPath(new URL("..", import.meta.url));
const args = process.argv.slice(2);
const dryRun = args.includes("--dry-run");
const allowBranch = args.includes("--allow-branch");
const version = args.find((a) => !a.startsWith("--"))?.replace(/^v/, "");
const tag = `v${version}`;

const sh = (cmd, cmdArgs, opts = {}) => execFileSync(cmd, cmdArgs, { cwd: root, encoding: "utf8", ...opts }).trim();
const fail = (msg) => {
  console.error(`✖ ${msg}`);
  process.exit(1);
};

if (!version || !isSemver(version)) fail("Usage: pnpm release <semver> [--dry-run] [--allow-branch]   e.g. pnpm release 0.2.0");
if (sh("git", ["status", "--porcelain"])) fail("Working tree is not clean. Commit or stash first.");
const branch = sh("git", ["rev-parse", "--abbrev-ref", "HEAD"]);
if (branch !== "main" && !allowBranch) fail(`Releases are cut from main (you are on ${branch}). Pass --allow-branch to override.`);
if (sh("git", ["tag", "-l", tag])) fail(`Tag ${tag} already exists.`);
const current = readVersions(root);
console.log(`Current versions: ${JSON.stringify(current)}`);
console.log(`Releasing ${tag}${dryRun ? " (dry run)" : ""}`);

const edits = [
  [VERSION_FILES.packageJson, (t) => setJsonVersion(t, version)],
  [VERSION_FILES.tauriConf, (t) => setJsonVersion(t, version)],
  [VERSION_FILES.cargoToml, (t) => setCargoTomlVersion(t, version)],
  [CARGO_LOCK, (t) => setCargoLockVersion(t, CRATE, version)],
];
const cliff = (cliffArgs) => sh("pnpm", ["exec", "git-cliff", ...cliffArgs], { stdio: ["ignore", "pipe", "inherit"] });
const notes = cliff(["--unreleased", "--tag", tag, "--strip", "all"]);

if (dryRun) {
  edits.forEach(([f]) => console.log(`  would bump ${f}`));
  console.log("  would regenerate CHANGELOG.md, commit and tag. Notes:\n");
  console.log(notes);
  process.exit(0);
}

for (const [file, edit] of edits) {
  const p = join(root, file);
  writeFileSync(p, edit(readFileSync(p, "utf8")));
}
cliff(["--tag", tag, "-o", "CHANGELOG.md"]);
sh("git", ["add", ...edits.map(([f]) => f), "CHANGELOG.md"]);
sh("git", ["commit", "-m", `chore(release): ${tag}`]);
sh("git", ["tag", "-a", tag, "-F", "-", "--cleanup=verbatim"], { input: `${tag}\n\n${notes}\n` });
console.log(`\n✔ Committed and tagged ${tag}. Publish with:\n\n  git push origin ${branch} --follow-tags\n`);
