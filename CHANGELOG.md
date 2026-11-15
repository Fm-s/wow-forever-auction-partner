# Changelog

All notable changes to WoW Forever Auction Partner are documented here.
This file is generated from commit messages by [git-cliff](https://git-cliff.org).

## [0.1.1](https://github.com/Fm-s/wow-forever-auction-partner/releases/tag/v0.1.1) — 2026-11-15

### 🐛 Fixes

- Keep items without a buyout last when sorting by min buyout ([`8d43c2d`](https://github.com/Fm-s/wow-forever-auction-partner/commit/8d43c2db37cf095ce025bff331df104e861d203b))

### 🏗️ Build & CI

- Bump the actions group with 4 updates ([`0654774`](https://github.com/Fm-s/wow-forever-auction-partner/commit/065477453944fe3ccfb607d308282429d7943bf6))

### 📦 Dependencies

- **deps:** Bump httpmock from 0.8.3 to 0.9.0 in /src-tauri ([`6ff8a60`](https://github.com/Fm-s/wow-forever-auction-partner/commit/6ff8a608511ade0c44b1898fba91ec1b28a5b7f7))
- **deps:** Bump happy-dom from 20.0.11 to 21.0.4 ([`95470db`](https://github.com/Fm-s/wow-forever-auction-partner/commit/95470db46abae926361cb9fe95a8e4ed2a3f3160))
- **deps:** TypeScript 7.0.2 and TanStack Table 9.3.1 ([`b271ca7`](https://github.com/Fm-s/wow-forever-auction-partner/commit/b271ca705137cc82a223a2dc50861dd2ed401cf9))


**Full diff:** [v0.1.0...v0.1.1](https://github.com/Fm-s/wow-forever-auction-partner/compare/v0.1.0...v0.1.1)
## [0.1.0](https://github.com/Fm-s/wow-forever-auction-partner/releases/tag/v0.1.0) — 2026-11-15

### 🚀 Features

- Add domain types and API fixtures ([`317da83`](https://github.com/Fm-s/wow-forever-auction-partner/commit/317da83536450a90069ec859c2cad392f71fc5bc))
- Parse Blizzard auction feeds ([`b925303`](https://github.com/Fm-s/wow-forever-auction-partner/commit/b925303e51b6a0e6bbabfd826dd5802b411c42f1))
- Parse AHledger price tables ([`dab469f`](https://github.com/Fm-s/wow-forever-auction-partner/commit/dab469f21b6c885bdf5a7f783dc2721b4af1dccc))
- Add Blizzard and AHledger HTTP clients ([`0e4d5e5`](https://github.com/Fm-s/wow-forever-auction-partner/commit/0e4d5e5f02d8abd797ebcef26e3fb625ff8e963c))
- Add SQLite schema and settings ([`dc222a8`](https://github.com/Fm-s/wow-forever-auction-partner/commit/dc222a84afff68a1a58684ad705ad3246a167da7))
- Per-snapshot aggregation and volume diff ([`4145a5f`](https://github.com/Fm-s/wow-forever-auction-partner/commit/4145a5fff37ed3681e2597f9b167f8c05b57b6f4))
- Ingest snapshots with rollups and retention ([`9e06c68`](https://github.com/Fm-s/wow-forever-auction-partner/commit/9e06c683bd7fc036c753dd418efec6514b62613f))
- Item metadata cache and credentials store ([`47f95fb`](https://github.com/Fm-s/wow-forever-auction-partner/commit/47f95fb9d70f1150c80642fc934b1fdabba16d0d))
- Background collector ([`6fab03e`](https://github.com/Fm-s/wow-forever-auction-partner/commit/6fab03ea24404390839a2f362de6f6ef84efb8ae))
- Market, history and health queries ([`60fdc86`](https://github.com/Fm-s/wow-forever-auction-partner/commit/60fdc86f8f1286f0bc421bb89a7ffe6c1ab174aa))
- Wire commands, tray and autostart ([`ddf28da`](https://github.com/Fm-s/wow-forever-auction-partner/commit/ddf28da94d5797a54568622cb09010ecfa780b20))
- App shell, money formatting and settings ([`1332955`](https://github.com/Fm-s/wow-forever-auction-partner/commit/13329557d53a74df90b884685ceefe202d362b61))
- Market browser and item detail ([`c2b6989`](https://github.com/Fm-s/wow-forever-auction-partner/commit/c2b6989f780dc85f34029abec69481b6fb3778fc))
- Health and sources view ([`04bec6f`](https://github.com/Fm-s/wow-forever-auction-partner/commit/04bec6f676f033d101516ff8c6e0c99417e06651))
- Item tooltips on icon hover, cached locally ([`76fd5ed`](https://github.com/Fm-s/wow-forever-auction-partner/commit/76fd5ed6e6aec9dd678d2011600c766ffeb4c8b1))
- Item group storage and commands ([`d133489`](https://github.com/Fm-s/wow-forever-auction-partner/commit/d133489833a37c1e46710fb9f4c5db63d0ab967f))
- Group filter, row context menu and group settings ([`d3c60dd`](https://github.com/Fm-s/wow-forever-auction-partner/commit/d3c60dd5d02f8713c5e1bec19fb47614e474b41b))
- Track any Forever realm type in US or EU ([`852d83c`](https://github.com/Fm-s/wow-forever-auction-partner/commit/852d83cd21ddced1f8bbe33da72cf0fae8244705))
- First-launch setup, realm settings and realm label ([`101f802`](https://github.com/Fm-s/wow-forever-auction-partner/commit/101f802763dc0aa105e16cff9baf7d70657afcc0))
- In-app updates from GitHub releases ([`2be11ce`](https://github.com/Fm-s/wow-forever-auction-partner/commit/2be11ced1c3f81256f4b5d04d3a77c71ef4cdf41))
- Rebrand as WoW Forever Auction Partner with a custom icon ([`0a03f47`](https://github.com/Fm-s/wow-forever-auction-partner/commit/0a03f47a6516a97ebdc7534b5521dc7656e0b0a4))

### 🐛 Fixes

- Keep cadence probe running through network errors ([`dc7ba46`](https://github.com/Fm-s/wow-forever-auction-partner/commit/dc7ba4631350006386bdcecb9aedf1c7648cb032))
- Address whole-branch review findings ([`1f17d5b`](https://github.com/Fm-s/wow-forever-auction-partner/commit/1f17d5b60fbae9afdff97fb5b8d644d7c25596bf))
- Group menu and popover dismissal, menu height, group id reuse ([`edc0dbe`](https://github.com/Fm-s/wow-forever-auction-partner/commit/edc0dbebc5996b11c1e28f95ba4b3803e4ad0728))
- Keep collecting after upgrade, keep the saved AHledger key ([`1c71e76`](https://github.com/Fm-s/wow-forever-auction-partner/commit/1c71e76529f32e6e57108f7af77f381453940aaa))
- **ci:** Fail releases on version mismatch, pin builds to one commit, full notes after betas ([`92d85f7`](https://github.com/Fm-s/wow-forever-auction-partner/commit/92d85f745ad25b9337b60e4280b9e875840c365d))
- Compile on Windows and Linux (Dock reopen is macOS-only) ([`cb63bf7`](https://github.com/Fm-s/wow-forever-auction-partner/commit/cb63bf7e4dfee29ad5f76491e461f5beb35cef8f))

### 📚 Documentation

- Add design spec and plan 1 ([`0125aa3`](https://github.com/Fm-s/wow-forever-auction-partner/commit/0125aa30b49a7c602c63750d09a31de625340567))
- Item groups design and plan ([`933a33e`](https://github.com/Fm-s/wow-forever-auction-partner/commit/933a33ee15ef8ffe418ab07374de1cd17a11c4d8))
- Realm setup design and plan ([`f8ffba5`](https://github.com/Fm-s/wow-forever-auction-partner/commit/f8ffba51b0f9d586a53cc809b4f91db52feabac9))
- Release pipeline design and plan ([`09c645e`](https://github.com/Fm-s/wow-forever-auction-partner/commit/09c645eb92e10548bb8c43686141501c8899cb59))
- README for users and releases ([`28c1e4a`](https://github.com/Fm-s/wow-forever-auction-partner/commit/28c1e4a74cfbdce37007dfb71a147e5d22ff9bc6))

### 🏗️ Build & CI

- Bundle for all platforms, real keyring backends ([`40cfd63`](https://github.com/Fm-s/wow-forever-auction-partner/commit/40cfd63f38679bec151ab7a3e0660ca76e7f7f18))
- Changelog generation and release command ([`8d114aa`](https://github.com/Fm-s/wow-forever-auction-partner/commit/8d114aa742e21a9b6f4ce5ac3df6becbbe412b0d))
- Release notes renderer ([`f7a9d3e`](https://github.com/Fm-s/wow-forever-auction-partner/commit/f7a9d3ea08df5786ccc2acbc03a7782236149214))
- Build, test and multi-platform release workflows ([`48c72ed`](https://github.com/Fm-s/wow-forever-auction-partner/commit/48c72ed0686a89db1c6709981658993463a3a07f))

### 🧹 Chores

- Add Blizzard snapshot cadence probe ([`f01bfa3`](https://github.com/Fm-s/wow-forever-auction-partner/commit/f01bfa36d1fc8877e848d063277a7ac327d127e6))
- Scaffold Tauri 2 + React app ([`066f449`](https://github.com/Fm-s/wow-forever-auction-partner/commit/066f4493958fa4e9107ed6d95516083d8c3f0f49))

### Merge

- Release pipeline (ci/release) into main line ([`82fcb89`](https://github.com/Fm-s/wow-forever-auction-partner/commit/82fcb89f3e3b881dd81cae56ff3291b107a95a31))



