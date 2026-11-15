#!/usr/bin/env node
// Renders the GitHub release body: changelog, downloads table, first-launch help, verification.
// CLI: node scripts/release-body.mjs --version 0.2.0 --repo owner/name --notes notes.md --assets dir/ [--prerelease]
import { createHash } from "node:crypto";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { assertSemver } from "./versions.mjs";

const PLATFORM_ORDER = ["macOS", "Windows", "Linux"];
const ARCH_ORDER = ["Universal (Apple Silicon + Intel)", "x64", "ARM64"];
// Platform, then architecture, then the order packages appear in KINDS (recommended first).
const rank = (info) => PLATFORM_ORDER.indexOf(info.platform) * 100 + ARCH_ORDER.indexOf(info.arch) * 10 + KINDS.findIndex(([, , kind]) => kind === info.kind);

const KINDS = [
  [/\.dmg$/i, "macOS", "Disk image (.dmg)"],
  [/-setup\.exe$/i, "Windows", "Installer (.exe)"],
  [/\.msi$/i, "Windows", "MSI package (.msi)"],
  [/\.AppImage$/, "Linux", "AppImage (any distro, auto-updates)"],
  [/\.deb$/i, "Linux", "Debian / Ubuntu (.deb)"],
  [/\.rpm$/i, "Linux", "Fedora / openSUSE (.rpm)"],
];

function archOf(name) {
  if (/universal/i.test(name)) return "Universal (Apple Silicon + Intel)";
  if (/(aarch64|arm64)/i.test(name)) return "ARM64";
  if (/(x64|amd64|x86_64)/i.test(name)) return "x64";
  return "—";
}

/** Platform/arch/kind of a user-facing installer, or null for updater bundles, signatures and metadata. */
export function classifyAsset(name) {
  for (const [re, platform, kind] of KINDS) if (re.test(name)) return { platform, arch: archOf(name), kind };
  return null;
}

const mb = (bytes) => `${(bytes / 1_000_000).toFixed(1)} MB`;

/** git-cliff renders its own "## [x.y.z](…) — date" heading; the release title already says that. */
/** The newest "## " section of a git-cliff changelog (header stripped). */
export function topSection(md) {
  const sections = md.trim().split(/^(?=## )/m);
  return (sections.find((s) => s.startsWith("## ")) ?? "").trim();
}

export const stripVersionHeading = (notes) => notes.replace(/^\s*## \[[^\]]+\].*\n+/, "").trim();

export function renderReleaseBody({ version, repo, notes, assets, prerelease }) {
  assertSemver(version);
  const tag = `v${version}`;
  const url = (name) => `https://github.com/${repo}/releases/download/${tag}/${encodeURIComponent(name)}`;
  const installers = assets
    .map((a) => ({ ...a, info: classifyAsset(a.name) }))
    .filter((a) => a.info)
    .sort((a, b) => rank(a.info) - rank(b.info) || a.name.localeCompare(b.name));
  const example = installers[0]?.name ?? "<file>";

  const out = [];
  if (prerelease) out.push("> [!WARNING]\n> **Pre-release.** For testing — the in-app updater only offers stable releases.\n");
  out.push(stripVersionHeading(notes) || "_No user-facing changes._", "");

  out.push("## ⬇️ Downloads", "", "| Platform | Architecture | Package | Size |", "|---|---|---|---|");
  for (const a of installers) out.push(`| ${a.info.platform} | ${a.info.arch} | [${a.info.kind}](${url(a.name)}) | ${mb(a.size)} |`);
  out.push("", "Already installed? Auctioneer offers the update in-app on its next launch.", "");

  out.push(
    "## 🚪 First launch",
    "",
    "These builds are not signed with an Apple or Microsoft certificate, so the OS asks once before the first launch.",
    "",
    "- **macOS:** open the `.dmg` and drag Auctioneer to Applications. On first launch macOS says it can't verify the app — open **System Settings → Privacy & Security**, scroll down and click **Open Anyway**. (Or run `xattr -dr com.apple.quarantine /Applications/Auctioneer.app`.) After each update macOS may ask once for your login password so Auctioneer can read its saved credentials — choose **Always Allow**.",
    "- **Windows:** SmartScreen shows “Windows protected your PC” — click **More info → Run anyway**.",
    "- **Linux:** `chmod +x Auctioneer_*.AppImage && ./Auctioneer_*.AppImage`, or install the `.deb` / `.rpm`. Credentials are stored via the Secret Service (GNOME Keyring / KWallet).",
    "",
  );

  out.push(
    "## 🔐 Verify your download",
    "",
    "Every file is checksummed and has a [build provenance attestation](https://docs.github.com/actions/security-for-github-actions/using-artifact-attestations) proving it was built by this repository's release workflow from this tag.",
    "",
    "```sh",
    "# checksums (download SHA256SUMS.txt next to your file)",
    "shasum -a 256 -c SHA256SUMS.txt --ignore-missing",
    "",
    "# provenance (GitHub CLI)",
    `gh attestation verify ${example} -R ${repo}`,
    "```",
    "",
  );

  out.push("<details><summary>SHA-256 checksums</summary>", "", "```", ...assets.map((a) => `${a.sha256}  ${a.name}`), "```", "", "</details>", "");
  return out.join("\n");
}

function sha256File(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function cli(argv) {
  const opt = (k) => {
    const i = argv.indexOf(`--${k}`);
    return i >= 0 ? argv[i + 1] : undefined;
  };
  const dir = opt("assets");
  const assets = readdirSync(dir)
    .filter((n) => statSync(join(dir, n)).isFile())
    .sort()
    .map((name) => ({ name, sha256: sha256File(join(dir, name)), size: statSync(join(dir, name)).size }));
  process.stdout.write(
    renderReleaseBody({ version: opt("version"), repo: opt("repo"), notes: readFileSync(opt("notes"), "utf8"), assets, prerelease: argv.includes("--prerelease") }),
  );
}

if (process.argv[1] === fileURLToPath(import.meta.url)) cli(process.argv.slice(2));
