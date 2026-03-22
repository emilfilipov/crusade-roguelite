# TASKS.md

## Planning Notes
- Board style: Jira-like backlog with task keys, dependencies, implementation steps, and acceptance criteria.
- Primary stack: Rust + Bevy.
- Primary target: Windows (`x86_64-pc-windows-msvc`).
- Distribution: Steam-ready, but local Windows installer is required from early milestones.
- Scope limits and expansion gates are tracked in `docs/SYSTEM_SCOPE_MAP.md`.

## Global Delivery Rules (Apply to Every Task)
1. Run full quality loop before closing a task:
   - `cargo fmt --all`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --all-targets --all-features`
   - `cargo build --release --target x86_64-pc-windows-msvc`
2. If any check/test/build fails, investigate and fix, then rerun the full loop until green.
3. Any logic that can be unit tested must have unit tests.
4. Do not close a task with failing tests.
5. After successful build/test loop, push changes to repository.
6. When expanding a previously limited system, update `docs/SYSTEM_SCOPE_MAP.md` and add/update task cards in this file.
7. Documentation-only changes (including `.md` files) must still be committed and pushed; no local-only markdown drift.

## Status Legend
- `TODO`: not started
- `IN PROGRESS`: active
- `BLOCKED`: waiting on dependency/decision
- `DONE`: implemented, tested, and pushed

---

## Active Backlog
- No active tasks at the moment.

---

## Recently Completed
### CRU-060 - Opposing-Faction Enemy Pool + Playable Muslim Scaffold
- Status: `DONE`
- Type: `Core`
- Priority: `P0`
- Depends on: `none`
- Goal: Make faction selection fully playable for both Christian and Muslim, and spawn enemies from the opposite faction pool.
- Context:
  - Match setup previously exposed Muslim as a disabled placeholder.
  - Enemy runtime previously used a single bandit enemy profile unrelated to player faction.
  - Unit/recruit data schema needed dual-faction coverage without breaking existing systems.
- Implementation:
  1. Expand model/data schema with Muslim commander/recruit kinds and mirrored enemy profile sets.
  2. Wire runtime systems (`squad`, `rescue`, `enemies`) to selected faction for friendlies and opposite faction for enemies.
  3. Enable Muslim in match setup and update upgrade-roster/UI helpers for faction-aware unit lists.
- Unit Tests Required:
  - Config load/validation tests for dual-faction units/enemies.
  - Runtime tests for faction selection availability and opposite-faction spawn plumbing.
- Acceptance Criteria:
  - Player can start runs as Christian or Muslim.
  - Enemy waves use the opposite faction pool for the selected player faction.
  - Rescue spawns pull from selected faction’s recruit pool only.
  - Full quality loop and installer packaging pass.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`

## Task Card Template
### CRU-XXX - <Title>
- Status: `TODO`
- Type: `<Gameplay|UI|Core|Balance|Release|QA|Docs>`
- Priority: `<P0|P1|P2>`
- Depends on: `<none|CRU-###,...>`
- Goal: `<one clear outcome sentence>`
- Context:
  - Why this task exists.
  - Runtime constraints or known pitfalls.
  - Exact files/systems expected to change.
- Implementation:
  1. `<step 1>`
  2. `<step 2>`
  3. `<step 3>`
- Unit Tests Required:
  - `<test case 1>`
  - `<test case 2>`
- Acceptance Criteria:
  - `<observable runtime result 1>`
  - `<observable runtime result 2>`
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md` (if scope gate changes)
  - `docs/ASSET_SOURCES.md` (if assets/source usage changes)
