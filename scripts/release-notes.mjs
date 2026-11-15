#!/usr/bin/env node
// Usage: node scripts/release-notes.mjs v1.2.3   → markdown notes for that release on stdout.
// Renders the full changelog (betas folded into stable releases) and keeps the newest section;
// git-cliff's --current/--latest would stop at the previous beta tag even with --ignore-tags.
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { topSection } from "./release-body.mjs";
import { assertSemver, cliffTagArgs } from "./versions.mjs";

const root = fileURLToPath(new URL("..", import.meta.url));

export function generateNotes(tag) {
  const version = tag.replace(/^v/, "");
  assertSemver(version);
  const md = execFileSync("pnpm", ["exec", "git-cliff", "--tag", tag, "--strip", "header", ...cliffTagArgs(version)], {
    cwd: root,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "inherit"],
  });
  return topSection(md);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) process.stdout.write(`${generateNotes(process.argv[2] ?? "")}\n`);
