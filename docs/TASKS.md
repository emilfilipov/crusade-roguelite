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
### CRU-060 - Unit Upgrade Modal Graph Redesign (Tier Columns + Scaffold)
- Status: `DONE`
- Type: `UI`
- Priority: `P0`
- Depends on: `<none>`
- Goal: Replace the current table-based Unit Upgrade modal with a tier-column graph layout that is readable and scaffold-ready for higher tiers.
- Context:
  - Current Unit Upgrade UI is table-centric and does not visually communicate tier progression.
  - The redesign must preserve existing conversion/promotion event plumbing while changing presentation and interaction affordances.
  - Files expected: `src/ui.rs`, `docs/SYSTEMS_REFERENCE.md`.
- Implementation:
  1. Replace Unit Upgrade panel layout with a graph-style tier presentation using thinly bordered tier columns.
  2. Render tier-0 unit boxes in the tier-0 column with hero scaffold node under tier-0 entries.
  3. Render tier-1..tier-5 as scaffolded/inactive boxes and draw straight visual connectors from tier-0 boxes to tier-1 boxes.
  4. Keep commander level budget and XP progression summary visible in the modal.
- Unit Tests Required:
  - Add/extend pure helper tests for graph row/column data shaping.
  - Preserve existing roster affordability tests.
- Acceptance Criteria:
  - Unit Upgrade modal shows clear tier columns and graph-like progression rows.
  - Tier-1+ and hero scaffold nodes are visible but inactive.
  - Existing in-run close/menu behavior remains functional.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`

### CRU-061 - Tier-0 Swap Controls Per Unit Row (Dropdown + Swap)
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-060`
- Goal: Expose per-tier0-unit conversion controls directly under each tier-0 type with selectable target and swap action.
- Context:
  - Swapping tier-0 units was hard to discover and unstable in previous iterations.
  - UX requires each tier-0 row to have its own target selector and swap trigger.
  - Files expected: `src/ui.rs`, `src/squad.rs` (event use remains), `docs/SYSTEMS_REFERENCE.md`.
- Implementation:
  1. Extend unit-upgrade UI state to persist per-source selected swap targets.
  2. Add target selector control (dropdown-like cycle selector) per tier-0 unit row.
  3. Add swap action button per tier-0 unit row that sends `ConvertTierZeroUnitsEvent`.
  4. Surface affordability/status text per row (owned count, target count, max affordable, XP cost).
- Unit Tests Required:
  - Unit test selected swap target resolution/fallback behavior.
  - Unit test affordability helper behavior for tier-0 swaps.
- Acceptance Criteria:
  - User can select source/target per tier-0 row and execute swap without reopening modal.
  - Swap controls respect XP and source-count gating.
  - Conversion feedback remains visible via existing feedback channel.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`

### CRU-062 - Unit Box Tooltip Contract (Name/Type/Description/Stats/Abilities)
- Status: `DONE`
- Type: `UI`
- Priority: `P0`
- Depends on: `CRU-060`
- Goal: Make every unit box in Unit Upgrade show only the unit name, with a hover tooltip providing full unit metadata.
- Context:
  - Requested UX is minimal box labels with rich hover details.
  - Tooltip must include unit name, role/type, description, key stats, and abilities.
  - Files expected: `src/ui.rs`, `src/data.rs` (read-only use), `docs/SYSTEMS_REFERENCE.md`.
- Implementation:
  1. Introduce unit-box tooltip components and tooltip overlay for Unit Upgrade modal.
  2. Bind hover detection on unit boxes to tooltip text updates and positioning.
  3. Render unit boxes with name-only labels; move all extra details into tooltip content.
  4. Provide scaffold tooltip content for inactive tier boxes and hero scaffold node.
- Unit Tests Required:
  - Unit test unit tooltip content builder for representative unit kinds.
- Acceptance Criteria:
  - Unit boxes show only unit names.
  - Hovering a unit box displays the required structured tooltip fields.
  - Tooltip hides when not hovering any unit box or when modal closes.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`

---

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
