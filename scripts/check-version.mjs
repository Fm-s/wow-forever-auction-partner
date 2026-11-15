#!/usr/bin/env node
// Usage: node scripts/check-version.mjs v1.2.3
// Fails when the tag isn't semver or any version file disagrees with it.
import { fileURLToPath } from "node:url";
import { assertVersionsMatch, isPrerelease, isSemver, readVersions } from "./versions.mjs";

const tag = process.argv[2] ?? "";
const version = tag.replace(/^v/, "");
if (!tag.startsWith("v") || !isSemver(version)) {
  console.error(`::error::Tag "${tag}" is not of the form v<semver> (e.g. v1.2.3 or v1.3.0-beta.1)`);
  process.exit(1);
}
try {
  assertVersionsMatch(readVersions(fileURLToPath(new URL("..", import.meta.url))), version);
} catch (e) {
  console.error(`::error::${e.message.replaceAll("\n", "%0A")}`);
  console.error("Run `pnpm release <version>` to bump every file, then re-tag.");
  process.exit(1);
}
console.log(`version=${version}`);
console.log(`prerelease=${isPrerelease(version)}`);
