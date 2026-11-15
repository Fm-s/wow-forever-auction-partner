# Release pipeline — design

Date: 2026-11-15 · Branch: `ci/release` · Repo: `Fm-s/wow-forever-auction-partner`

## Intent

Auctioneer is going public: other WoW Forever players should be able to download it for
macOS, Windows and Linux from GitHub Releases, verify what they downloaded, and receive
updates inside the app. Releasing must be one command plus a tag push.

## Goals

- Tag push `v*` → full multi-platform release with no manual steps.
- Every asset has a SHA-256 checksum and a GitHub build-provenance attestation.
- Release notes and `CHANGELOG.md` generated from Conventional Commits (repo already uses them).
- In-app auto-update from GitHub Releases, signed with the Tauri updater key.
- CI on every push/PR catches cross-platform breakage before release time.

## Non-goals

- OS code signing / notarization (no Apple Developer ID, no Windows cert). User decision.
- App store distribution, Homebrew/winget/Flathub manifests.

## Anti-goals

- A release that publishes half the platforms: the release stays a draft until every build succeeds.
- Credentials silently not persisting on Windows/Linux (keyring backend must be real).
- Prereleases pushed to stable users via the updater.

## Approaches considered

| Approach | Trade-off | Verdict |
|---|---|---|
| Tag-triggered `tauri-apps/tauri-action` matrix + git-cliff | Standard Tauri path, full control of notes, local release script | **Chosen** — user asked for tag trigger |
| release-please (bot PR bumps version + changelog, merging tags) | Hands-off, but PR-driven flow; user commits directly | Rejected |
| GitHub auto-generated release notes | Zero config, but built from PR titles — empty for direct commits | Rejected |

## Design

### `release.yml` (on `push: tags: v*`, plus `workflow_dispatch` for dry runs)

1. **verify** (ubuntu): tag version == `package.json` == `tauri.conf.json` == `Cargo.toml`;
   `pnpm build` (tsc + vite), vitest, `cargo test`. Generates release notes for the tag with
   git-cliff (`--latest --strip header`), exposes them as an artifact.
2. **create-release**: creates a **draft** GitHub release with the notes. Prerelease when the
   tag contains `-`.
3. **build** matrix (fail-fast off), each via `tauri-action` uploading to the draft:
   - macOS `universal-apple-darwin` → `.dmg` + `.app.tar.gz` (updater)
   - Windows x64 (`windows-latest`) and arm64 (`windows-11-arm`) → NSIS `.exe`, `.msi`
   - Linux x64 (`ubuntu-22.04`) and arm64 (`ubuntu-22.04-arm`) → `.AppImage`, `.deb`, `.rpm`
   - Tauri updater signatures (`.sig`) and `latest.json` produced by tauri-action.
   - Rust + pnpm caches.
4. **publish**: downloads all release assets, writes `SHA256SUMS.txt`, runs
   `actions/attest-build-provenance` on every asset, generates an SPDX SBOM, uploads both,
   rewrites the release body (changelog + downloads/checksum table + install/verify
   instructions for unsigned builds), then un-drafts.

### `ci.yml` (push to any branch, PRs)

- Frontend: `pnpm build`, vitest.
- Rust: `cargo fmt --check`, `cargo clippy`, `cargo test` on ubuntu; `cargo check` on macOS and
  Windows.

### Local release command

`pnpm release <version>`: validates clean tree + semver, bumps the three version files and
`Cargo.lock`, regenerates `CHANGELOG.md` via git-cliff, commits `chore(release): v<version>`,
creates annotated tag. Prints the push command; does not push.

### App changes

- `keyring` features: add `windows-native`, `sync-secret-service`, `crypto-rust`
  (Linux persistent Secret Service; builds need `libdbus-1-dev`).
- `bundle.targets: "all"`; `bundle.macOS.signingIdentity: "-"` (ad-hoc).
- `bundle.createUpdaterArtifacts: true`; `plugins.updater` with pubkey and endpoint
  `https://github.com/Fm-s/wow-forever-auction-partner/releases/latest/download/latest.json`
  (GitHub "latest" excludes prereleases → prereleases never auto-update stable users).
- `tauri-plugin-updater` + `tauri-plugin-process` (relaunch); capability permissions.
- `UpdateBanner` React component: checks on startup, shows version + notes, "Install &
  restart". Self-contained file + one mount line to minimise conflicts with `feat/item-groups`.

### Repo hygiene

- `cliff.toml`: groups Features / Fixes / Performance / Refactor / Docs / Build & CI;
  commit links to GitHub; `chore(release)` commits skipped.
- `.github/dependabot.yml`: github-actions, cargo, npm — weekly, grouped.
- README rewritten: what it is, downloads, install on unsigned builds, verify checksums and
  attestations, release process, required secrets.

## Secrets (user adds in GitHub → Settings → Secrets → Actions)

- `TAURI_SIGNING_PRIVATE_KEY` — content of `~/.tauri/auctioneer-updater.key`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — its password

Losing the private key means existing installs can no longer auto-update.

## Left to the builder

Exact action versions, cache keys, release-body layout, banner styling (match existing app
styles).
