<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="128" height="128" alt="">
</p>

<h1 align="center">WoW Forever Auction Partner</h1>

<p align="center">
  <a href="https://github.com/Fm-s/wow-forever-auction-partner/actions/workflows/ci.yml"><img src="https://github.com/Fm-s/wow-forever-auction-partner/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/Fm-s/wow-forever-auction-partner/actions/workflows/release.yml"><img src="https://github.com/Fm-s/wow-forever-auction-partner/actions/workflows/release.yml/badge.svg" alt="Release"></a>
  <a href="https://github.com/Fm-s/wow-forever-auction-partner/releases/latest"><img src="https://img.shields.io/github/v/release/Fm-s/wow-forever-auction-partner?sort=semver" alt="Latest release"></a>
</p>

A desktop auction-house companion for **World of Warcraft: Forever**. It runs quietly in your tray, collects every
Blizzard auction snapshot for your realm type's Alliance, Horde and Blackwater houses, and builds its own price and
volume history so you can spot what's cheap, what sells, and where.

Works with every Forever realm type (**PvP, Normal, Roleplay, Roleplay PvP**) in the **US and EU** regions. You pick
one at first launch and can switch in Settings; each realm keeps its own history.

- **Market browser:** every item per house with min buyout, median, quantity, 7-day median and estimated sales per day
- **Item detail:** price and quantity history charts and a cross-house comparison
- **Item tooltips:** hover an icon for the in-game tooltip (stats, effects, set bonuses, sell price), cached locally
- **Groups:** right-click any item to file it into your own groups (Flips, Herbs, …) and filter the market by them
- **Second opinion:** prices checked against [AHledger](https://ahledger.com), with disagreements flagged
- **Data health:** last fetch per source, snapshot gaps, errors
- **Tray app:** starts at login, collects in the background, updates itself

Sales per day are estimates from listings that disappear between snapshots, not Blizzard sales data.

## Download

Get the latest version from **[Releases](https://github.com/Fm-s/wow-forever-auction-partner/releases/latest)**:

| Platform | File |
|---|---|
| macOS 11+ (Apple Silicon and Intel) | `…_universal.dmg` |
| Windows 10/11 x64 · ARM64 | `…_x64-setup.exe` · `…_arm64-setup.exe` (or the `.msi`) |
| Linux x64 · ARM64 | `.AppImage` (any distro, auto-updates), `.deb`, `.rpm` |

Once installed, the app checks for new versions at launch and offers **Install & restart**.

### First launch

Builds are not signed with an Apple or Microsoft certificate, so your OS asks once:

- **macOS:** drag WoW Forever Auction Partner to Applications and open it. When macOS says it can't verify the
  developer, open **System Settings → Privacy & Security**, scroll down and click **Open Anyway**.
  Alternatively: `xattr -dr com.apple.quarantine "/Applications/WoW Forever Auction Partner.app"`
  After an update, macOS may ask once for your login password so the app can read its saved credentials.
  Choose **Always Allow**. (Unsigned builds get a new code signature every release.)
- **Windows:** on “Windows protected your PC”, click **More info → Run anyway**.
- **Linux:** make the `.AppImage` executable (`chmod +x`) and run it, or install the `.deb` / `.rpm`. Credentials
  are stored through the Secret Service (GNOME Keyring or KWallet), which must be running.

### Setup

The app reads Blizzard's official Game Data API with your own free API client. On first launch it asks for:

1. **Your realm:** region (US or EU) and realm type (PvP, Normal, Roleplay or Roleplay PvP).
2. **Blizzard API access:** sign in at [develop.battle.net](https://develop.battle.net/access/clients), create a
   client (any name and redirect URL), and paste its **Client ID** and **Client Secret**. They're checked with
   Blizzard before they're saved to your OS keychain.

An [AHledger key](https://ahledger.com/developers) is optional; it only raises AHledger's rate limit.

### Verify your download

Every release ships a `SHA256SUMS.txt` and a GitHub
[build provenance attestation](https://docs.github.com/actions/security-for-github-actions/using-artifact-attestations)
for each file, proving it was built by this repository's release workflow from the tagged commit.

```sh
shasum -a 256 -c SHA256SUMS.txt --ignore-missing
gh attestation verify <downloaded file> -R Fm-s/wow-forever-auction-partner
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
