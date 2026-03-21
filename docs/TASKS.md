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
### CRU-060 - Level-Up Header + Tier Legend
- Status: `DONE`
- Type: `UI`
- Priority: `P1`
- Depends on: `none`
- Goal: Make the level-up screen use the short title and add a visible rarity legend for card border colors.
- Context:
  - Current header text is verbose and includes a subtitle the user requested to remove.
  - Rarity colors exist but there is no in-screen legend for interpretation.
  - Scope: `src/ui.rs`.
- Implementation:
  1. Replace level-up header text with `Level Up!`.
  2. Remove the subtitle line from the level-up modal.
  3. Add a compact rarity legend row with color-coded bordered squares and labels in rarity order.
- Unit Tests Required:
  - Add/adjust UI construction test covering rarity color mapping ordering.
- Acceptance Criteria:
  - Level-up modal top title is exactly `Level Up!`.
  - Rarity guide is visible and uses the same border colors as cards.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`

### CRU-061 - Retinue Spawn Rate Increase
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P1`
- Depends on: `none`
- Goal: Increase rescue/retinue spawn frequency so rescues appear more often.
- Context:
  - Rescue respawn timing is currently too slow relative to current combat pacing.
  - Scope: `src/rescue.rs`, tests for spawn interval behavior.
- Implementation:
  1. Reduce rescue respawn interval constant and keep max-active guard intact.
  2. Update any interval-expectation unit tests.
  3. Verify rescue pacing in runtime does not exceed active cap behavior.
- Unit Tests Required:
  - Update rescue interval test to new interval.
  - Keep spawn pool/tier tests green.
- Acceptance Criteria:
  - Rescuable units repopulate faster than prior build.
  - Rescue max-active rules still hold.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`

### CRU-062 - Double Attack-Speed Upgrade Values
- Status: `DONE`
- Type: `Balance`
- Priority: `P1`
- Depends on: `none`
- Goal: Increase attack-speed upgrade impact by doubling offered values.
- Context:
  - User feedback: attack-speed card impact feels too low.
  - Scope: `assets/data/upgrades.json`, upgrade tests if needed.
- Implementation:
  1. Double `attack_speed_up` min/max/step values while preserving weighted roll behavior.
  2. Keep value-tier mapping valid with fixed 5 weighted tiers.
  3. Revalidate upgrade data loading tests.
- Unit Tests Required:
  - Ensure weighted tier tests remain valid.
  - Ensure upgrade JSON validation remains green.
- Acceptance Criteria:
  - Attack-speed cards show doubled value ranges versus previous build.
  - Random roll behavior is unchanged except value magnitude.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`

### CRU-063 - Critical Hit System + Upgrade Entries
- Status: `DONE`
- Type: `Core`
- Priority: `P0`
- Depends on: `none`
- Goal: Add crit chance and crit damage as combat stats, wire to combat, and expose as level-up upgrades.
- Context:
  - Requested new combat stats and card options.
  - Must apply to melee and ranged friendly attacks.
  - Scope: `src/model.rs`, `src/combat.rs`, `src/upgrades.rs`, `assets/data/upgrades.json`, UI/stat presentation where relevant.
- Implementation:
  1. Extend `GlobalBuffs` with crit stats and defaults.
  2. Add deterministic/runtime-safe crit roll utility and apply to outgoing friendly attacks.
  3. Add upgrade kinds (`crit_chance`, `crit_damage`) with icons/titles/descriptions and application logic.
  4. Surface crit stats in stats/skill-book cumulative descriptions.
- Unit Tests Required:
  - Crit roll threshold behavior.
  - Crit damage multiplier application in damage path.
  - Upgrade metadata mapping for new upgrade kinds.
- Acceptance Criteria:
  - Friendly attacks can critically hit based on chance.
  - Crit damage multiplier affects hit damage correctly.
  - Crit upgrades appear in level-up pool and stack additively.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md` (if scope wording needs expansion)

### CRU-064 - Add Squire Equipment Slot (All Unit Setups)
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P1`
- Depends on: `none`
- Goal: Add a `Squire` equipment slot to commander and all tier setups.
- Context:
  - User requested slot scaffolding now; bonus wiring can remain neutral unless item defines effects.
  - Scope: `src/inventory.rs`, inventory UI rendering in `src/ui.rs`.
- Implementation:
  1. Append `Squire` slot to commander and non-commander default slot lists.
  2. Ensure UI grid rendering displays the extra slot cleanly.
  3. Keep current slot-bonus routing (melee/ranged/armor explicit) unchanged.
- Unit Tests Required:
  - Default setup slot count/name assertions.
  - Inventory serde round-trip test updates.
- Acceptance Criteria:
  - Every equipment setup includes `Squire`.
  - Inventory UI shows the new slot for commander and all tiers.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`

### CRU-065 - Hero Tier Scaffolding
- Status: `DONE`
- Type: `Core`
- Priority: `P1`
- Depends on: `CRU-064`
- Goal: Scaffold a `Hero` tier in systems/UI without enabling gameplay progression into it yet.
- Context:
  - Requirement is explicit scaffold only.
  - Scope: inventory tier enums, labels, UI rendering, and any tier conversion helpers.
- Implementation:
  1. Add `Hero` to equipment-tier model/enums and setup generation.
  2. Add short UI label and placement for hero tier equipment row.
  3. Keep upgrade unlock logic unchanged (no active hero progression path yet).
- Unit Tests Required:
  - Tier enum mapping tests include Hero.
  - Default inventory setup includes Hero row.
- Acceptance Criteria:
  - Hero tier appears in equipment UI and state.
  - No runtime progression path changes are introduced unintentionally.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`

### CRU-066 - Peasant Priest Visual Replacement
- Status: `TODO`
- Type: `UI`
- Priority: `P1`
- Depends on: `none`
- Goal: Replace potion placeholder for peasant priest with a character sprite.
- Context:
  - Current priest sprite uses potion art and is visually incorrect.
  - Scope: `src/visuals.rs` (asset mapping), optional `docs/ASSET_SOURCES.md`.
- Implementation:
  1. Select an existing character tile from current third-party packs suitable for priest/support.
  2. Update priest idle asset handle path.
  3. Verify distinguishability from infantry/archer/enemy sprites.
- Unit Tests Required:
  - Asset path references compile/runtime load in current tests.
- Acceptance Criteria:
  - Priest is rendered as a unit character, not a potion icon.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/ASSET_SOURCES.md`

### CRU-067 - Wave Cap and Enemy Stat Scaling Adjustment
- Status: `TODO`
- Type: `Balance`
- Priority: `P0`
- Depends on: `none`
- Goal: Keep current spawn model but cap per-wave enemy count at 1000 and increase wave stat growth by +15%.
- Context:
  - User requested explicit cap and stronger scaling.
  - Scope: `src/enemies.rs` and tests.
- Implementation:
  1. Add hard cap constant for enemies-per-wave and apply in units-per-second computation.
  2. Increase wave stat growth constant by 15% from current value.
  3. Update/extend tests for capped throughput and revised stat scaling.
- Unit Tests Required:
  - Per-wave cap test.
  - Stat multiplier progression test with new growth constant.
- Acceptance Criteria:
  - Effective spawned count per wave cannot exceed 1000.
  - Enemy stat multiplier grows faster than before by the requested amount.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`

### CRU-068 - Formation Footprint Occupancy Cap + Perimeter Repel
- Status: `TODO`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-067`
- Goal: Limit how many enemies can remain inside formation footprint and push overflow toward perimeter.
- Context:
  - Required to prevent overwhelming inner stacking and preserve formation readability.
  - Scope: enemy movement/control systems and formation math helpers.
- Implementation:
  1. Compute dynamic max-allowed inside-footprint count from formation size/recruit count.
  2. Identify overflow enemies inside footprint and steer/repel them to nearest boundary perimeter.
  3. Keep behavior deterministic and stable across low/high FPS.
- Unit Tests Required:
  - Perimeter target computation for square/diamond.
  - Overflow selection respects cap.
- Acceptance Criteria:
  - Inside-footprint enemy count is capped in practice.
  - Overflow enemies are visibly displaced toward perimeter rather than stacking inward.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`

### CRU-069 - Enemy Stacking/Jitter Mitigation Pass
- Status: `TODO`
- Type: `Core`
- Priority: `P0`
- Depends on: `CRU-068`
- Goal: Reduce high-density enemy overlap and jitter under heavy load.
- Context:
  - Prior snapshots show severe overlap and FPS collapse at later waves.
  - Scope: `src/collision.rs` and potentially enemy movement smoothing constants.
- Implementation:
  1. Tune collision correction damping/push/frame-scale clamps for low-FPS stability.
  2. Revalidate enemy-enemy collision handling remains active and performant.
  3. Add/adjust tests for solver convergence under larger frame deltas.
- Unit Tests Required:
  - Updated solver stability tests.
  - Collision rule test remains green.
- Acceptance Criteria:
  - Enemy clumping is reduced in dense battles.
  - No major jitter regressions introduced.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`

### CRU-070 - Magnet Pickup Drop System
- Status: `TODO`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-067`
- Goal: Add faction-symbol magnet pickup that force-homes all active XP packs to commander.
- Context:
  - Spawn cadence: once every 3 waves, at wave start, centered map position, despawn on next wave.
  - Visual requirement: cross for Christian, crescent for Muslim on world and minimap.
  - Scope: `src/drops.rs`, `src/ui.rs`, `src/visuals.rs` (if assets required), faction read from `MatchSetupSelection`.
- Implementation:
  1. Add magnet pickup entity/components/runtime wave lifecycle management.
  2. On pickup, mark all XP packs as transit-to-commander immediately.
  3. Add world symbol rendering and minimap symbol rendering based on selected faction.
- Unit Tests Required:
  - Wave lifecycle helper logic (spawn/expire cadence).
  - Magnet trigger forces XP packs into homing state.
- Acceptance Criteria:
  - Magnet appears on waves divisible by 3 and disappears on next wave transition.
  - Picking magnet immediately force-homes all active XP packs.
  - Magnet has faction symbol in world and minimap.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/ASSET_SOURCES.md` (if new assets are added)

### CRU-071 - Minimap + Utility Bar Layout Swap and Minimap +20%
- Status: `TODO`
- Type: `UI`
- Priority: `P1`
- Depends on: `none`
- Goal: Increase minimap size by 20% and swap its position with the top-right utility action bar.
- Context:
  - Requested layout refinement for readability and quick access.
  - Scope: `src/ui.rs`.
- Implementation:
  1. Increase minimap size constant by 20%.
  2. Move minimap container to utility bar’s old location.
  3. Move utility bar to minimap’s old location while preserving interaction behavior.
- Unit Tests Required:
  - Minimap coordinate mapping tests updated for new size constant.
- Acceptance Criteria:
  - Minimap renders larger (+20%).
  - Minimap and utility bar are swapped in-screen positions.
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
