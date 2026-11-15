# Auctioneer

[![CI](https://github.com/Fm-s/wow-forever-auction-partner/actions/workflows/ci.yml/badge.svg)](https://github.com/Fm-s/wow-forever-auction-partner/actions/workflows/ci.yml)
[![Release](https://github.com/Fm-s/wow-forever-auction-partner/actions/workflows/release.yml/badge.svg)](https://github.com/Fm-s/wow-forever-auction-partner/actions/workflows/release.yml)
[![Latest release](https://img.shields.io/github/v/release/Fm-s/wow-forever-auction-partner?sort=semver)](https://github.com/Fm-s/wow-forever-auction-partner/releases/latest)

A desktop auction-house companion for **World of Warcraft: Forever** (PvP, US). Auctioneer runs quietly in your
tray, collects every Blizzard auction snapshot for the Alliance, Horde and Blackwater houses, and builds its own
price and volume history so you can spot what's cheap, what sells, and where.

- **Market browser:** every item per house with min buyout, median, quantity, 7/30-day medians and estimated sales per day
- **Item detail:** price and quantity history charts and cross-house comparison
- **Second opinion:** prices checked against [AHledger](https://ahledger.com), with disagreements flagged
- **Data health:** last fetch per source, snapshot gaps, errors
- **Tray app:** starts at login, collects in the background, updates itself

## Download

Get the latest version from **[Releases](https://github.com/Fm-s/wow-forever-auction-partner/releases/latest)**:

| Platform | File |
|---|---|
| macOS 11+ (Apple Silicon and Intel) | `Auctioneer_<version>_universal.dmg` |
| Windows 10/11 x64 · ARM64 | `Auctioneer_<version>_x64-setup.exe` · `_arm64-setup.exe` (or the `.msi`) |
| Linux x64 · ARM64 | `.AppImage` (any distro, auto-updates), `.deb`, `.rpm` |

Once installed, Auctioneer checks for new versions at launch and offers **Install & restart**.

### First launch

Builds are not signed with an Apple or Microsoft certificate, so your OS asks once:

- **macOS:** drag Auctioneer to Applications and open it. When macOS says it can't verify the developer, open
  **System Settings → Privacy & Security**, scroll down and click **Open Anyway**.
  Alternatively: `xattr -dr com.apple.quarantine /Applications/Auctioneer.app`
  After an update, macOS may ask once for your login password so Auctioneer can read its saved credentials.
  Choose **Always Allow**. (Unsigned builds get a new code signature every release.)
- **Windows:** on “Windows protected your PC”, click **More info → Run anyway**.
- **Linux:** `chmod +x Auctioneer_*.AppImage` and run it, or install the `.deb` / `.rpm`. Credentials are stored
  through the Secret Service (GNOME Keyring or KWallet), which must be running.

### Setup

Auctioneer reads Blizzard's official Game Data API with your own free API client:

1. Sign in at [develop.battle.net](https://develop.battle.net/access/clients) and create a client.
2. Paste its **Client ID** and **Client Secret** into Auctioneer → **Settings**. They are stored in your OS keychain.

### Verify your download

Every release ships a `SHA256SUMS.txt` and a GitHub
[build provenance attestation](https://docs.github.com/actions/security-for-github-actions/using-artifact-attestations)
for each file, proving it was built by this repository's release workflow from the tagged commit.

```sh
shasum -a 256 -c SHA256SUMS.txt --ignore-missing
gh attestation verify Auctioneer_<version>_universal.dmg -R Fm-s/wow-forever-auction-partner
```

In-app updates are additionally signed with the project's updater key and rejected if the signature doesn't match.

## Development

Requirements: Node 22+, pnpm, Rust stable, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/)
for your OS (on Debian/Ubuntu, `.github/scripts/linux-deps.sh` installs them).

```sh
pnpm install
pnpm tauri dev      # run the app
pnpm test           # frontend + release-script tests
cd src-tauri && cargo test
```

## Releasing

Commits follow [Conventional Commits](https://www.conventionalcommits.org) (`feat:`, `fix:`, `perf:`, …); the
changelog and release notes are generated from them with [git-cliff](https://git-cliff.org).

```sh
pnpm release 0.2.0 --dry-run   # preview the notes
pnpm release 0.2.0             # bump versions, update CHANGELOG.md, commit, tag v0.2.0
git push origin main --follow-tags
```

Pushing the tag runs [`release.yml`](.github/workflows/release.yml):

1. **Verify:** the tag matches `package.json`, `tauri.conf.json` and `Cargo.toml`, and the frontend and Rust tests pass
2. **Draft:** a draft GitHub release is created with the generated notes
3. **Build:** macOS universal, Windows x64/ARM64 and Linux x64/ARM64 build in parallel, and updater bundles are signed
4. **Publish:** `latest.json`, `SHA256SUMS.txt`, an SPDX SBOM and provenance attestations are added, the release
   page gets downloads and verification instructions, and the draft is published

If any platform fails, the release stays a draft. For a flaky failure, **re-run the failed jobs**; every job builds
the exact commit `verify` checked, so a re-run can't mix commits. For a code fix, delete the draft and the tag,
fix on `main`, then tag again. Tags with a suffix (`v0.3.0-beta.1`) publish as pre-releases, which the in-app
updater ignores; the next stable release's notes include everything since the previous stable.

### Required repository secrets

| Secret | Value |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | contents of the updater private key (`pnpm tauri signer generate`) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | its password |

The matching public key is in `src-tauri/tauri.conf.json` → `plugins.updater.pubkey`. **Back up the private key.**
If it's lost, existing installs can no longer verify updates and users must reinstall manually.

## Credits

Price comparison data from [AHledger](https://ahledger.com). World of Warcraft is a trademark of Blizzard
Entertainment; this project is not affiliated with or endorsed by Blizzard.
