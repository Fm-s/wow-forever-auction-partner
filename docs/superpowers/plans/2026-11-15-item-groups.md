# Item Groups Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Named, shared groups of item variants, managed from a Market row context menu and Settings, filtering the Market table by a union of groups.

**Architecture:** Two SQLite tables (migration 4) behind five Tauri commands in `src-tauri/src/groups.rs`; App holds the group list and the selected group ids; the Market screen filters its rows client-side by the union of member keys.

**Tech Stack:** Rust (rusqlite, thiserror), React 19 + TS, vitest.

**Spec:** `docs/superpowers/specs/2026-11-15-item-groups-design.md`

## Global Constraints

- Membership key is `(item_id, variant_key)`; variant_key `""` for items without variants.
- Group names: trimmed, 1–60 chars, unique case-insensitively. Errors: `Group name can't be empty`, `Group names are at most 60 characters`, `A group named <name> already exists`.
- Commands run DB work via the existing `with_db` helper (never on the main thread).
- No browser storage for groups.

## Review Focus

- Renaming a group to its own name with different case must succeed (uniqueness excludes itself).
- Adding a member twice / removing a non-member must be no-ops, not errors.
- Deleting a group must remove its members (foreign keys are ON in `db::prepare`).
- A selected group that gets deleted must not leave the Market filtered to nothing.
- The context menu must stay inside the window near the right/bottom edges.

---

### Task 1: Storage and commands

**Files:** Create `src-tauri/src/db/migration_4.sql`, `src-tauri/src/groups.rs`; modify `src-tauri/src/db/mod.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/commands.rs`.

**Produces:**
- `Group { id: i64, name: String, members: Vec<GroupMember> }`, `GroupMember { item_id: i64, variant_key: String }` (serde camelCase)
- `list_groups(&Connection) -> rusqlite::Result<Vec<Group>>` (ordered by name, case-insensitive)
- `create_group(&Connection, &str, now: i64) -> Result<Group, GroupError>`
- `rename_group(&Connection, id: i64, &str) -> Result<(), GroupError>`
- `delete_group(&Connection, id: i64) -> rusqlite::Result<()>`
- `set_member(&Connection, group_id, item_id, &str variant_key, member: bool, now) -> rusqlite::Result<()>`
- Commands `get_groups`, `create_group`, `rename_group`, `delete_group`, `set_group_member`.

Tests (`groups::tests`): `create_trims_and_lists_sorted`, `names_are_validated`, `duplicate_names_rejected_ignoring_case`, `rename_to_own_name_in_other_case_is_allowed`, `membership_is_per_variant_and_idempotent`, `delete_removes_members`.

### Task 2: Filter logic

**Files:** Modify `src/marketFilter.ts`, `src/marketFilter.test.ts`, `src/types.ts`, `src/api.ts`.

**Produces:** `memberKey(itemId, variantKey): string`; `groupUnion(groups: Group[], selected: number[]): Set<string> | null` (null = no filter; ignores ids that no longer exist); `MarketFilter.groupKeys: Set<string> | null`.

Tests: union of two groups; empty selection → null; only-deleted ids → null; `filterRows` keeps only member variants.

### Task 3: UI

**Files:** Create `src/components/GroupMenu.tsx`, `src/components/GroupFilter.tsx`, `src/menuPosition.ts` (+ test); modify `src/App.tsx`, `src/views/MarketView.tsx`, `src/views/SettingsView.tsx`, `src/App.css`.

- `clampMenu(x, y, w, h, vw, vh, edge = 8): {left, top}` keeps the menu inside the window (tests: near right edge flips left of the cursor; near bottom shifts up).
- Context menu on Market rows; Groups popover in the toolbar; Groups section in Settings.

Verify: `npx vitest run`, `npx tsc --noEmit`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, headless screenshots of the menu, the filter and Settings.
