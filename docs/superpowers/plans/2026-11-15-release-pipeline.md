# Release Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Tag push `v*` produces a public, multi-platform, checksummed, attested GitHub release with generated notes, and the app updates itself from it.

**Architecture:** Pure Node helpers in `scripts/` (version bumping, release-body rendering) are unit-tested with vitest and called from a local `pnpm release` command and from GitHub Actions. `release.yml` = verify → draft release → tauri-action build matrix → publish (checksums, attestations, SBOM, body, un-draft). App gains `tauri-plugin-updater` + a self-contained `UpdateBanner`.

**Tech Stack:** GitHub Actions, tauri-apps/tauri-action v0, git-cliff, Node 22 ESM scripts, vitest, Tauri 2 updater/process plugins.

**Spec:** `docs/superpowers/specs/2026-11-15-release-pipeline-design.md`

## Global Constraints

- Repo slug: `Fm-s/wow-forever-auction-partner`. Updater endpoint: `https://github.com/Fm-s/wow-forever-auction-partner/releases/latest/download/latest.json`.
- Tags: `v<semver>`; a `-` in the tag ⇒ prerelease.
- Version lives in `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` (+ `Cargo.lock` entry for `auctioneer`); all must equal the tag.
- Secrets: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Private key file `~/.tauri/auctioneer-updater.key`, never in repo.
- No OS code signing; macOS `signingIdentity: "-"`.
- Do not run `cargo fmt` over the codebase (collides with `feat/item-groups`); CI does not gate on fmt; clippy warnings are not errors.
- Touch `App.tsx`, `lib.rs` minimally (one mount line / plugin lines) — another branch edits them.

## Review Focus

1. Tag/version mismatch (e.g. tag `v0.2.0`, files at `0.1.0`) must fail verify before any build — test in Task 3 (`readVersions` mismatch).
2. Prerelease tag must not become GitHub "latest" (would push betas to updater) — `make_latest: false` + test of `isPrerelease` in Task 3.
3. One platform build failing must leave the release a draft — publish job `needs: build` with no `if: always()`; verified in Task 5 review.
4. Update with unknown content length (no `Content-Length`) — `progressPercent` returns `null`, banner shows indeterminate; test in Task 2.
5. Asset names with spaces/parentheses in checksum table — `renderReleaseBody` URL-encodes links; test in Task 4.

---

### Task 1: Cross-platform bundling and credential storage

**Files:**
- Modify: `src-tauri/Cargo.toml` (keyring features)
- Modify: `src-tauri/tauri.conf.json` (`bundle`)

- [ ] **Step 1:** keyring → `features = ["apple-native", "windows-native", "sync-secret-service", "crypto-rust"]`.
- [ ] **Step 2:** `bundle.targets: "all"`, add `bundle.macOS: { "signingIdentity": "-", "minimumSystemVersion": "11.0" }`, `bundle.publisher`, `shortDescription`, `longDescription`, `category: "Utility"`, `copyright`, `homepage` (repo URL), `linux.deb.depends: []` default.
- [ ] **Step 3:** Verify `cd src-tauri && cargo test` passes (all existing tests).
- [ ] **Step 4:** Commit `build: bundle for all platforms, real keyring backends`.

### Task 2: In-app updater

**Files:**
- Create: `src/updateState.ts`, `src/updateState.test.ts`, `src/components/UpdateBanner.tsx`
- Modify: `package.json` (`@tauri-apps/plugin-updater`, `@tauri-apps/plugin-process`), `src-tauri/Cargo.toml` (`tauri-plugin-updater = "2"`, `tauri-plugin-process = "2"`), `src-tauri/src/lib.rs` (2 `.plugin(...)` lines; updater under `#[cfg(desktop)]` via `.setup` or builder), `src-tauri/capabilities/default.json` (`updater:default`, `process:allow-restart`), `src-tauri/tauri.conf.json` (`bundle.createUpdaterArtifacts: true`, `plugins.updater.{pubkey, endpoints, windows.installMode: "passive"}`), `src/App.tsx` (mount `<UpdateBanner />` under header), `src/App.css` (banner styles using existing tokens).

**Interfaces:**
- Produces: `progressPercent(downloaded: number, total: number | null | undefined): number | null` — integer 0–100, clamped; `null` when total unknown/0.
- Produces: `type UpdatePhase = "idle" | "available" | "downloading" | "ready" | "error"`.

- [ ] **Step 1:** Generate key: `pnpm tauri signer generate -w ~/.tauri/auctioneer-updater.key -p <random>`; store password in `~/.tauri/auctioneer-updater.password` (chmod 600). Pubkey goes in config.
- [ ] **Step 2: Failing tests** in `src/updateState.test.ts`: `progressPercent(50, 200) === 25`; `progressPercent(10, null) === null`; `progressPercent(10, 0) === null`; `progressPercent(300, 200) === 100`; `progressPercent(1, 3) === 33`.
- [ ] **Step 3:** Run `pnpm test` → FAIL (module missing). Implement. Run → PASS.
- [ ] **Step 4:** `UpdateBanner`: on mount `check()`; errors swallowed to `console.warn` (offline must not show an error); when update available show "Auctioneer v{version} is available" + collapsible notes + "Install & restart" → `downloadAndInstall` with progress → `relaunch()`; "Later" dismisses for the session. Skip check entirely in `import.meta.env.DEV`.
- [ ] **Step 5:** `pnpm build` and `cd src-tauri && cargo check` pass.
- [ ] **Step 6:** Commit `feat: in-app updates from GitHub releases`.

### Task 3: Version tooling, changelog and `pnpm release`

**Files:**
- Create: `scripts/versions.mjs`, `scripts/versions.test.mjs`, `scripts/release.mjs`, `scripts/check-version.mjs`, `cliff.toml`, `CHANGELOG.md`
- Modify: `package.json` (`"release": "node scripts/release.mjs"`, devDep `git-cliff`, `packageManager`)

**Interfaces (`scripts/versions.mjs`):**
- `isSemver(v: string): boolean`
- `isPrerelease(v: string): boolean` — true iff semver has `-pre` part
- `setJsonVersion(text: string, v: string): string` — replaces top-level `"version"` only, preserves formatting
- `setCargoTomlVersion(text: string, v: string): string` — only the `[package]` `version`
- `setCargoLockVersion(text: string, crate: string, v: string): string` — only that crate's `[[package]]` block
- `readVersions(root: string): { packageJson, tauriConf, cargoToml }`
- `assertVersionsMatch(versions, expected: string): void` — throws listing each mismatching file

- [ ] **Step 1: Failing tests:** `isSemver("1.2.3")`, `isSemver("1.2.3-beta.1")` true; `isSemver("v1.2")` false; `isPrerelease("1.0.0-rc.1")` true / `"1.0.0"` false; `setCargoTomlVersion` changes `[package]` version but not `tauri = { version = "2" }`; `setCargoLockVersion` changes only `name = "auctioneer"` block; `setJsonVersion` leaves nested `"version"` keys alone; `assertVersionsMatch({packageJson:"0.1.0",tauriConf:"0.2.0",cargoToml:"0.2.0"}, "0.2.0")` throws mentioning `package.json`.
- [ ] **Step 2:** `pnpm test` → FAIL; implement; → PASS (ensure vitest includes `scripts/**`).
- [ ] **Step 3:** `check-version.mjs <tag>`: strips `v`, `readVersions` + `assertVersionsMatch`, exits 1 with message.
- [ ] **Step 4:** `release.mjs <version>`: requires clean tree + on `main` (override `--allow-branch`), tag not existing, then bump 4 files, `git cliff --tag v<version> -o CHANGELOG.md`, commit `chore(release): v<version>`, annotated tag `v<version>` with that version's notes; prints `git push origin main --follow-tags`. `--dry-run` prints actions only.
- [ ] **Step 5:** `cliff.toml`: conventional parsing, groups (🚀 Features, 🐛 Fixes, ⚡ Performance, ♻️ Refactor, 📚 Docs, 🧪 Tests, 🏗️ Build & CI, 🧹 Chores), skip `chore(release)`, commit links to the repo, `[remote.github]` set. Generate `CHANGELOG.md` with unreleased section; check `pnpm exec git-cliff --unreleased` output lists the item-group-free history from main.
- [ ] **Step 6:** Commit `build: changelog generation and release command`.

### Task 4: Release body renderer

**Files:**
- Create: `scripts/release-body.mjs`, `scripts/release-body.test.mjs`

**Interfaces:**
- `renderReleaseBody({ version: string, repo: string, notes: string, assets: {name: string, sha256: string, size: number}[], prerelease: boolean }): string`
- CLI: `node scripts/release-body.mjs --version X --repo R --notes notes.md --assets dir/ > body.md` (hashes files itself).

- [ ] **Step 1: Failing tests:** body contains notes verbatim; a downloads table where `.dmg` row is under macOS, `.exe`/`.msi` under Windows, `.AppImage`/`.deb`/`.rpm` under Linux, with arch derived from name (`aarch64`/`arm64` → ARM64, `x64`/`amd64`/`x86_64` → x64, `universal` → Universal); `.sig`, `latest.json`, `.tar.gz` updater files excluded from the downloads table but present in the checksums block; asset `Auctioneer (beta).dmg` link is `encodeURIComponent`-encoded; prerelease adds a "⚠️ Pre-release" banner; includes install-on-unsigned sections and `gh attestation verify` + `shasum -a 256 -c` instructions.
- [ ] **Step 2:** FAIL → implement → PASS.
- [ ] **Step 3:** Commit `build: release notes renderer`.

### Task 5: GitHub workflows

**Files:**
- Create: `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.github/dependabot.yml`

- [ ] **Step 1: ci.yml** — triggers: push (all branches), pull_request; concurrency cancel-in-progress. Jobs: `frontend` (pnpm build + test), `rust` matrix ubuntu-22.04/macos-latest/windows-latest: linux deps, rust-cache, `cargo clippy --all-targets` (non-fatal warnings) on ubuntu, `cargo test` on all. Needs `dist/` dir to exist for `generate_context!` (run `pnpm build` or `mkdir dist`).
- [ ] **Step 2: release.yml** — `on: push: tags: ['v*']` + `workflow_dispatch` (input `tag`). `permissions: contents: write, id-token: write, attestations: write`. Jobs:
  - `verify`: checkout fetch-depth 0, `check-version.mjs`, pnpm build/test, cargo test (ubuntu), git-cliff `--latest --strip header` → `notes.md` artifact; outputs `version`, `prerelease`.
  - `create-release`: draft via `gh release create --draft --verify-tag --notes-file`, `--prerelease` when needed; output release id.
  - `build` matrix (`fail-fast: false`): macos-latest `--target universal-apple-darwin` (rust targets aarch64+x86_64), windows-latest, windows-11-arm, ubuntu-22.04, ubuntu-22.04-arm. tauri-action with `releaseId`, `includeUpdaterJson: true`, `updaterJsonPreferNsis: true`, signing env secrets.
  - `publish` (needs build): `gh release download` all assets → `SHA256SUMS.txt`; `actions/attest-build-provenance` with `subject-path` all assets; `anchore/sbom-action` SPDX JSON; upload both; `release-body.mjs` → `gh release edit --notes-file body.md --draft=false` (+ `--latest=false` for prerelease).
- [ ] **Step 3: dependabot.yml** — github-actions, cargo (`/src-tauri`), npm (`/`), weekly, grouped minor/patch.
- [ ] **Step 4:** Lint with `actionlint` (brew or docker) → no errors.
- [ ] **Step 5:** Commit `ci: build, test and multi-platform release workflows`.

### Task 6: README

- [ ] **Step 1:** Replace template README: what it is, downloads (link to latest release), first-launch steps (macOS Open Anyway / `xattr -dr com.apple.quarantine`, SmartScreen "More info → Run anyway", AppImage `chmod +x`), verify (`shasum -a 256 -c SHA256SUMS.txt --ignore-missing`, `gh attestation verify <file> -R Fm-s/wow-forever-auction-partner`), development, releasing (`pnpm release X` → push), required secrets, CI badges.
- [ ] **Step 2:** Commit `docs: README for users and releases`.

### Task 7: Publish branches

- [ ] **Step 1:** Full verification: `pnpm test`, `pnpm build`, `cargo test`, actionlint.
- [ ] **Step 2:** `git push -u origin main` and `git push -u origin ci/release`. No tag.
