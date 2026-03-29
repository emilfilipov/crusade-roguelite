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

### Revamp Track: Major/Minor Upgrade + Deterministic Itemization
- Execution order (high level): `CRU-200 -> CRU-218 -> CRU-201 -> CRU-202 -> CRU-203 -> (CRU-213 + CRU-214 + CRU-215) -> (CRU-204 + CRU-205) -> CRU-206 -> CRU-207 -> CRU-208 -> CRU-217 -> CRU-209 -> CRU-216 -> CRU-210 -> CRU-211 -> CRU-212`.
- Scope intent: replace mild incremental progression with strategic doctrine choices, remove roll-tier dependence from upgrades/items, and preserve `level 100 @ wave 98`.

### CRU-200 - Revamp Design Contract and Balance Targets
- Status: `DONE`
- Type: `Docs`
- Priority: `P0`
- Depends on: `none`
- Goal: Freeze an implementation-ready design contract for the major/minor revamp before code/data changes begin.
- Context:
  - The current system still contains many globally-stacking mild upgrades and roll-tier driven value ranges.
  - Implementation must preserve deterministic progression pacing (`level 100` by end of `wave 98`).
  - Files expected to change: `docs/SYSTEMS_REFERENCE.md`, `docs/SYSTEM_SCOPE_MAP.md`, `docs/requirements.md`.
- Implementation:
  1. Define canonical lane taxonomy: `Minor` (frequent support picks) and `Major` (level-milestone doctrine picks with explicit downside).
  2. Define cadence contract: minor reward per level-up, major reward every `5` levels, while preserving `level 100` by end of wave `98`.
  3. Define tradeoff rules and minimum impact thresholds for all new major upgrades and deterministic items.
  4. Define accepted data invariants (no upgrade/item tier-roll value ranges, no rarity roll dependencies in upgrade logic).
- Unit Tests Required:
  - `none (docs-only task)`
  - `none (docs-only task)`
- Acceptance Criteria:
  - Revamp contract is documented with exact cadence, impact bands, and tradeoff requirements.
  - Team can implement all downstream cards without unresolved design ambiguity.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/requirements.md`

### CRU-201 - Upgrade and Item Schema Migration (Remove Tier-Roll Fields)
- Status: `DONE`
- Type: `Core`
- Priority: `P0`
- Depends on: `CRU-200`
- Goal: Replace roll-range based upgrade/item schema with deterministic authored fields that support major/minor design.
- Context:
  - Current upgrade config relies on `min_value/max_value/value_step/weight_exponent`.
  - Current item generation uses rarity roll weights and scalarized rolled stats.
  - Files expected to change: `src/data.rs`, `assets/data/upgrades.json`, item data source (`src/inventory.rs` and/or new `assets/data/items.json`).
- Implementation:
  1. Extend `UpgradeConfig` schema with lane/type metadata (major/minor, doctrine tags, stack caps, downside fields).
  2. Remove runtime dependence on upgrade roll-range fields and upgrade rarity value-roll math.
  3. Introduce deterministic item schema (fixed stat packages + explicit downside/tradeoff metadata).
  4. Harden validators to reject legacy roll-tier fields once migration is complete.
- Unit Tests Required:
  - Config validator rejects legacy roll-only fields after migration.
  - Config loader accepts new deterministic major/minor upgrade and deterministic item schema.
- Acceptance Criteria:
  - Runtime can load migrated data with no tier-roll fields required.
  - Invalid legacy schema is rejected with actionable validation errors.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md` (if scope language changes)
  - `docs/ASSET_SOURCES.md` (if new data assets are added)

### CRU-202 - Progression Reward Stream: Minor Per Level, Major Every 5 Levels
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-201, CRU-218`
- Goal: Implement deterministic reward queue that delivers minor picks each level-up and major picks at every `5`-level milestone without breaking level pacing.
- Context:
  - Current progression queue supports level rewards from wave completion and checkpoint bonus levels.
  - New cadence must coexist with existing `level 100 @ wave 98` rule.
  - Files expected to change: `src/upgrades.rs`, `src/enemies.rs`, `src/model.rs`.
- Implementation:
  1. Add explicit reward-kind queue model (minor vs major) and event path from level-up events.
  2. Encode level-based policy (`major when level % 5 == 0`) with deterministic handling for wave `98` bonus-level bursts.
  3. Ensure reward queue drain opens correct draft mode/state and never starves queued rewards.
  4. Preserve existing level cap/budget locks while consuming pending rewards.
- Unit Tests Required:
  - Level-to-reward schedule test confirms expected minor/major counts and wave `98` behavior.
  - Queue drain order test ensures mixed pending rewards resolve deterministically.
- Acceptance Criteria:
  - Level progression still reaches `100` by wave `98` completion.
  - Major rewards appear only at `5`-level milestones per contract.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md` (if event/data diagrams are added)

### CRU-203 - Draft Engine Rewrite for Major/Minor Lanes
- Status: `DONE`
- Type: `Core`
- Priority: `P0`
- Depends on: `CRU-202`
- Goal: Replace rarity-roll draft behavior with lane-aware deterministic draft rules that enforce strategic option diversity.
- Context:
  - Current draft path is weighted by one-time vs repeatable and value-roll rarity.
  - New system must provide reliable doctrine choice quality and avoid dead drafts.
  - Files expected to change: `src/upgrades.rs`, `src/ui.rs`.
- Implementation:
  1. Split draft pools by lane (`major`, `minor`) and route by reward kind.
  2. Implement option composition rules: one synergy option, one pivot option, one safe option minimum.
  3. Keep uniqueness/stack cap gates and skillbar-cap gates compatible with lane rules.
  4. Remove mythical/value-tier visualization dependencies from draft generation.
- Unit Tests Required:
  - Lane routing test ensures major rewards never pull from minor pool (and vice versa).
  - Draft composition test verifies synergy/pivot/safe constraints are respected.
- Acceptance Criteria:
  - Drafts are lane-correct and composition-correct under deterministic seed replay.
  - No remaining runtime dependency on value-tier rarity rolls.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md` (if UI assets for lane labels are added)

### CRU-213 - Major Upgrade Design Catalogue and Ability Matrix
- Status: `DONE`
- Type: `Docs`
- Priority: `P0`
- Depends on: `CRU-203`
- Goal: Produce a complete design catalogue for major upgrades covering high-impact stat increases, specific unit/commander bonuses, and active abilities (formations, auras, temporary auto-cast buffs, and other tactical actives) with explicit tradeoffs.
- Context:
  - Major upgrades must be deliberately sparse and build-defining under level-milestone cadence.
  - Design must include both passive and active-style majors, not only global stats.
  - Files expected to change: `docs/SYSTEMS_REFERENCE.md`, `docs/SYSTEM_SCOPE_MAP.md`, design notes section in `docs/TASKS.md`.
- Implementation:
  1. Define major-upgrade category matrix with required buckets:
     - broad stat spikes,
     - unit-specific bonuses (infantry/archer/priest),
     - commander-specific bonuses,
     - formation actives,
     - aura actives/modifiers,
     - temporary auto-cast buffs,
     - other tactical active abilities.
  2. For each major candidate, author:
     - effect payload,
     - downside/tradeoff,
     - intended doctrine/archetype,
     - anti-synergy notes.
  3. Set target counts per category and per-run expected exposure under `every 5 levels` cadence.
  4. Define exclusion rules to prevent contradictory or degenerate major combinations.
- Unit Tests Required:
  - `none (design/docs task)`
  - `none (design/docs task)`
- Acceptance Criteria:
  - Approved major-upgrade matrix exists and explicitly includes stat increases, specific unit bonuses, formation actives, aura actives, temporary auto-cast buffs, and other active abilities.
  - Every major candidate has explicit upside, downside, and doctrine tag.
  - Implementation cards can directly consume catalogue entries without additional design clarification.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md` (if planned icon/FX requirements are listed)

### CRU-214 - Minor Upgrade Design Catalogue and Support Ability Matrix
- Status: `DONE`
- Type: `Docs`
- Priority: `P0`
- Depends on: `CRU-203`
- Goal: Produce a complete design catalogue for minor upgrades covering meaningful stat increases, specific unit/commander support bonuses, and support active abilities (formations, auras, temporary auto-cast buffs, and other utility actives).
- Context:
  - Minor upgrades must still matter but should not override major doctrine identity.
  - Minor design should include support actives and utility hooks, not only flat stat lines.
  - Files expected to change: `docs/SYSTEMS_REFERENCE.md`, `docs/SYSTEM_SCOPE_MAP.md`, design notes section in `docs/TASKS.md`.
- Implementation:
  1. Define minor-upgrade category matrix with required buckets:
     - moderate stat increases,
     - unit-specific support bonuses,
     - commander support bonuses,
     - formation support modifiers,
     - aura support modifiers,
     - temporary auto-cast support buffs,
     - other low-impact active utility abilities.
  2. For each minor candidate, author:
     - effect payload,
     - stack cap/diminishing rules,
     - synergy targets with major doctrines.
  3. Define minor pick pacing expectations and category distribution per run.
  4. Define pruning rules for redundant or non-decision-forming minor cards.
- Unit Tests Required:
  - `none (design/docs task)`
  - `none (design/docs task)`
- Acceptance Criteria:
  - Approved minor-upgrade matrix exists and explicitly includes stat increases, specific unit bonuses, formation support actives, aura support actives, temporary auto-cast support buffs, and other active utility abilities.
  - Every minor candidate has stack/diminishing policy and doctrine synergy mapping.
  - Minor catalogue is implementation-ready for content and balance cards.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md` (if planned icon/FX requirements are listed)

### CRU-215 - Qualitative Stat Bands and Descriptor Contract
- Status: `DONE`
- Type: `Docs`
- Priority: `P0`
- Depends on: `CRU-203`
- Goal: Define the player-facing stat-band model so primary UI communicates strength via bars and descriptors instead of raw numbers.
- Context:
  - Planned UX direction uses qualitative labels (for example `Low`, `Moderate`, `High`, `Very High`) with visual bars.
  - Upgrade/item text must align with this model (`+1 damage band`, `at least Moderate`) while preserving hidden numeric balance logic.
  - Files expected to change: `docs/SYSTEMS_REFERENCE.md`, `docs/SYSTEM_SCOPE_MAP.md`, `docs/requirements.md`.
- Implementation:
  1. Define canonical stat-band ladders per stat family (damage, durability/health, armor, speed, utility) and visual scale rules.
  2. Define descriptor text standards and trait keyword format for unit advantages/disadvantages.
  3. Define band-threshold mapping contract from internal numeric values to displayed bands.
  4. Define where exact numeric values remain available (`advanced details` surfaces only).
- Unit Tests Required:
  - `none (design/docs task)`
  - `none (design/docs task)`
- Acceptance Criteria:
  - A single approved stat-band taxonomy exists and is referenced by gameplay and UI tasks.
  - Descriptor and trait keyword format is unambiguous and reusable across units, items, and upgrades.
  - Exact-value visibility policy is explicitly documented for primary vs advanced UI surfaces.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/requirements.md`

### CRU-204 - Major Upgrade Content Pass (Commander/Unit Doctrine Cards)
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-213`
- Goal: Author high-impact major upgrades that define playstyle and force meaningful tradeoffs.
- Context:
  - Current major-feeling cards are limited and sparse (`mob_*`, formation unlock, formation breach).
  - Doctrine cards should target commander identity and unit-role identity (infantry/archer/priest).
  - Files expected to change: `assets/data/upgrades.json`, `src/upgrades.rs`, `src/combat.rs`, `src/morale.rs`, `src/squad.rs`.
- Implementation:
  1. Add doctrine families (for example: control, sustain, execute, tempo, anti-ranged, cavalry-pressure analogs).
  2. Author commander-specific major cards with large upside plus explicit downside.
  3. Author unit-role majors that create composition lock-in and counterplay tradeoffs.
  4. Enforce one-downside minimum validator for all major cards.
- Unit Tests Required:
  - Major upgrade validator test fails cards without downside/tradeoff metadata.
  - Runtime effect test confirms major card upside and downside are both applied.
- Acceptance Criteria:
  - Every major card materially changes build behavior and has a clear cost.
  - Runs can produce distinct doctrine identities from major picks alone.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md` (if new major-card icon assets are introduced)

### CRU-205 - Minor Upgrade Library Rebalance (Support Picks with Caps)
- Status: `DONE`
- Type: `Balance`
- Priority: `P1`
- Depends on: `CRU-214`
- Goal: Rebuild minor pool into meaningful support decisions without runaway universal stacking.
- Context:
  - Existing repeatables are mostly global and can converge into generic best-in-slot progression.
  - Minors should reinforce doctrine, patch weaknesses, and obey stack caps.
  - Files expected to change: `assets/data/upgrades.json`, `src/upgrades.rs`, `src/model.rs`.
- Implementation:
  1. Convert existing mild globals into doctrine-tagged support upgrades where possible.
  2. Add per-upgrade stack caps and diminishing-return rules for universally strong effects.
  3. Retune baseline values upward enough to matter within limited pick budget.
  4. Remove or merge redundant minors that do not change gameplay decisions.
- Unit Tests Required:
  - Stack cap test verifies capped minors stop increasing after limit.
  - Diminishing-return test verifies final effective bonus respects cap curve.
- Acceptance Criteria:
  - Minor picks feel relevant but do not invalidate major doctrine choices.
  - Generic stat stacking no longer produces dominant all-purpose builds.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md` (if card/icon set changes)

### CRU-206 - Conditional Upgrade Requirement Rework (Mob Line and Beyond)
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P1`
- Depends on: `CRU-204, CRU-205`
- Goal: Make conditional upgrades require real strategic commitment instead of default-on activation.
- Context:
  - Current `tier0_share` conditions can be trivially met in present roster structure.
  - Conditional effects should serve doctrine commitment and comp identity.
  - Files expected to change: `assets/data/upgrades.json`, `src/upgrades.rs`, `src/squad.rs`, `src/ui.rs`.
- Implementation:
  1. Redefine requirement thresholds/types to track real commitment states.
  2. Add clearer status messaging in UI for active/inactive reasons.
  3. Rebalance conditional rewards to match stricter requirements.
  4. Add guardrails to prevent contradictory conditional stacking exploits.
- Unit Tests Required:
  - Requirement evaluator tests for each new requirement type/threshold.
  - Conditional status text test confirms accurate active/inactive reasoning.
- Acceptance Criteria:
  - Conditional cards activate only under deliberate build conditions.
  - Players can clearly read why a conditional effect is active or inactive.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-207 - Deterministic Itemization Rebuild (Major/Minor Item Nature)
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-201`
- Goal: Replace rarity-rolled items with fixed authored item identities and explicit tradeoffs.
- Context:
  - Current item power comes from rarity/stat rolls and roll multipliers.
  - New item model should mirror strategic doctrine design: strong purpose, clear downside.
  - Files expected to change: `src/inventory.rs`, item data source, `src/ui.rs`, `src/archive.rs`.
- Implementation:
  1. Replace stat roll generation with deterministic item definitions per item line.
  2. Introduce item tags (doctrine/role/faction affinity) and explicit downside fields.
  3. Remove item-rarity roll bonus dependency from runtime item generation.
  4. Update inventory/chest/archive tooltip text to show deterministic effects and tradeoffs.
- Unit Tests Required:
  - Item generation test confirms deterministic stats/effects for each item template.
  - Item tooltip/metadata test confirms downside and doctrine info is surfaced.
- Acceptance Criteria:
  - Two players receiving same item get identical effects.
  - Items are strategically distinct and non-interchangeable by design intent.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-208 - Gold Economy and Scrap Rebalance for Deterministic Items
- Status: `TODO`
- Type: `Balance`
- Priority: `P1`
- Depends on: `CRU-207`
- Goal: Rebalance gold flow and costs so deterministic items/upgrades create real opportunity costs.
- Context:
  - Scrap value currently derives from rarity/stat tiers; this must change with deterministic itemization.
  - Unit swap/promotion and chest decisions must remain meaningful across full run.
  - Files expected to change: `src/inventory.rs`, `src/drops.rs`, `src/squad.rs`, `assets/data/drops.json`.
- Implementation:
  1. Replace scrap formula with deterministic valuation based on authored item class/value tier.
  2. Rebalance ambient/drop gold, swap costs, promotion costs, and chest value bands.
  3. Verify economy supports strategic tradeoffs (equip now vs scrap now vs save for roster actions).
  4. Add anti-snowball clamps if deterministic item value spikes create runaway loops.
- Unit Tests Required:
  - Scrap valuation test confirms stable deterministic value per item template.
  - Economy affordability test validates key decisions across early/mid/late wave bands.
- Acceptance Criteria:
  - Gold decisions remain meaningful throughout run and do not collapse into one dominant strategy.
  - No economy exploit path yields unchecked runaway purchasing.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-217 - Upgrade/Item Semantics Migration to Band Shifts and Trait Hooks
- Status: `TODO`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-204, CRU-205, CRU-207, CRU-215`
- Goal: Convert upgrade and item effects to player-facing qualitative semantics (`band shifts`, `floors`, and trait interactions) while keeping deterministic internal math.
- Context:
  - Current upgrade/item descriptions and effect framing are mostly numeric and do not communicate strategic role quickly.
  - Absolute wording like `set to Moderate` must be limited to explicit floor/fallback mechanics to avoid dead picks.
  - Files expected to change: `assets/data/upgrades.json`, item data source, `src/upgrades.rs`, `src/inventory.rs`, `src/combat.rs`, `src/ui.rs`.
- Implementation:
  1. Introduce explicit effect primitives for `+N band`, `minimum band floor`, and trait-conditional modifiers.
  2. Migrate upgrade/item definitions from raw stat deltas to qualitative semantics where player-facing clarity improves decisions.
  3. Add current-to-after preview metadata support so cards/tooltips can show band shifts.
  4. Add validators that reject ambiguous/degenerate wording patterns and unsupported trait hooks.
- Unit Tests Required:
  - Effect resolution tests for band shift, band floor, and trait-conditional modifier behavior.
  - Validator tests for illegal semantic combinations (for example conflicting floor + cap rules).
- Acceptance Criteria:
  - Upgrade/item catalog uses consistent qualitative semantics for strategic choices.
  - Runtime behavior remains deterministic and mathematically equivalent to intended authored values.
  - Cards can express `current -> after` outcomes for supported band-shift effects.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-209 - UI/UX Revamp for Laned Drafts and Tradeoff Clarity
- Status: `TODO`
- Type: `UI`
- Priority: `P0`
- Depends on: `CRU-202, CRU-203, CRU-207`
- Goal: Update level-up and inventory UI to clearly communicate major/minor lanes, level-milestone rewards, and downside tradeoffs.
- Context:
  - Current UI still references rarity/value-tier framing and generic card readability assumptions.
  - Strategic system requires explicit tradeoff readability in one glance.
  - Files expected to change: `src/ui.rs`, `src/archive.rs`, UI art hooks in `src/visuals.rs`.
- Implementation:
  1. Add lane labels and level-milestone framing to draft UI (`Minor` vs `Major` presentation).
  2. Display explicit upside/downside blocks on card face and tooltip.
  3. Remove obsolete rarity-tier card visuals and replace with doctrine/category visuals.
  4. Update inventory/chest context to display deterministic item role/tradeoff metadata.
- Unit Tests Required:
  - UI state test ensures `5`-level milestone rewards open major draft view.
  - Tooltip rendering test confirms downside text is always present for major options/items.
- Acceptance Criteria:
  - Player can immediately distinguish pick lane and tradeoff implications.
  - No leftover UI paths depend on removed rarity-roll tier semantics.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-216 - UI Conversion to Stat Bands and Trait-First Readability
- Status: `TODO`
- Type: `UI`
- Priority: `P0`
- Depends on: `CRU-209, CRU-215, CRU-217`
- Goal: Replace primary raw-number stat presentation with band bars, qualitative descriptors, and trait highlights across unit, inventory, and upgrade surfaces.
- Context:
  - The design goal is fast strategic readability for builds and unit roles, not spreadsheet-style stat parsing.
  - Exact values should remain available only in optional advanced details surfaces for players who want precision.
  - Files expected to change: `src/ui.rs`, `src/archive.rs`, `src/inventory.rs`, `src/visuals.rs`.
- Implementation:
  1. Add reusable stat-band widget (`|||||` style bars + descriptor text) and shared color/state conventions.
  2. Replace primary raw-number fields with band+descriptor fields in unit cards, unit detail panes, and inventory item views.
  3. Surface explicit trait advantages/disadvantages (for example block chance, anti-low-armor) in standardized keyword rows.
  4. Add `Advanced Details` toggle/panel exposing exact numeric values without cluttering primary HUD flow.
- Unit Tests Required:
  - UI rendering state tests for band widget and descriptor mapping at all threshold boundaries.
  - UI mode tests ensuring exact numbers are hidden in primary view and visible in advanced view.
- Acceptance Criteria:
  - Primary gameplay-facing UI is qualitative-first and readable at a glance.
  - Trait interactions are visibly surfaced wherever they affect decisions.
  - Players can still access exact values through advanced details without breaking baseline readability.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-210 - Integrated Balance Pass for Strategic Contrast
- Status: `TODO`
- Type: `Balance`
- Priority: `P0`
- Depends on: `CRU-204, CRU-205, CRU-206, CRU-208, CRU-209, CRU-216, CRU-217`
- Goal: Tune all major/minor upgrades and deterministic items into stable, high-contrast strategic archetypes.
- Context:
  - Power can shift dramatically once major cards and deterministic items are fully active.
  - Need cross-system tuning: combat, morale, movement, economy, and roster decisions.
  - Files expected to change: gameplay data files and tuning constants across `src/upgrades.rs`, `src/inventory.rs`, `src/squad.rs`, `src/combat.rs`, `src/morale.rs`.
- Implementation:
  1. Define target archetypes and expected strengths/weaknesses for each.
  2. Run controlled balance iterations across full run lengths and both factions/commander options.
  3. Tune outliers and enforce minimum strategic differentiation thresholds.
  4. Freeze v1 numbers and add patch-note style changelog block in docs.
- Unit Tests Required:
  - Regression tests for key formula invariants (caps, floors, clamp behavior) after rebalance.
  - Simulation-style tests for progression/economy sanity across early-mid-late runs.
- Acceptance Criteria:
  - Multiple archetypes are viable and materially different.
  - No single doctrine line dominates across both factions and commander selections.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-211 - Regression Harness and QA Matrix for Revamp
- Status: `TODO`
- Type: `QA`
- Priority: `P0`
- Depends on: `CRU-210`
- Goal: Lock in automated and manual validation coverage for the new strategic progression model.
- Context:
  - Revamp touches progression, combat scaling, morale dynamics, items, UI, and economy.
  - Needs robust guardrails before future feature expansion.
  - Files expected to change: tests in `src/*`, QA docs/checklists.
- Implementation:
  1. Add unit/integration tests for lane routing, level-milestone cadence, deterministic items, and tradeoff application.
  2. Add deterministic seed-based replay checks for draft reproducibility and economy sanity.
  3. Build manual QA matrix for faction/commander/doctrine permutations.
  4. Define pass/fail thresholds for release-readiness of the revamp branch.
- Unit Tests Required:
  - End-to-end progression test from wave `1` to `98` validating reward cadence and level target.
  - End-to-end doctrine test validating upside/downside both affect runtime behavior.
- Acceptance Criteria:
  - Critical revamp pathways are covered by stable automated tests.
  - QA matrix passes for both factions and all commander choices.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-212 - Release Integration and Documentation Finalization
- Status: `TODO`
- Type: `Release`
- Priority: `P1`
- Depends on: `CRU-211`
- Goal: Finalize revamp branch for release packaging, migration notes, and downstream maintainability.
- Context:
  - Revamp removes old conceptual dependencies (tier rolls for upgrades/items).
  - All docs and packaging flows must represent the new system accurately.
  - Files expected to change: `docs/*`, installer scripts/metadata if needed, release notes source.
- Implementation:
  1. Finalize documentation consistency pass across systems/scope/requirements/tasks.
  2. Confirm installer packaging includes any new data assets or UI resources.
  3. Add migration notes for removed mechanics and replaced tuning fields.
  4. Produce release checklist sign-off artifact for the revamp milestone.
- Unit Tests Required:
  - Packaging smoke test ensures runtime boots with migrated deterministic data.
  - Doc-reference sanity check (manual) verifies no stale roll-tier references remain.
- Acceptance Criteria:
  - Revamp is releasable with green CI and packaging flow.
  - Documentation accurately matches shipped behavior and new tuning workflow.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

---

## Army/Unit Expansion Backlog

### Army/Tier Expansion Track: Adaptive Enemy Armies + Tiered Roster + Hero Recruitment
- Execution order (high level): `CRU-218 -> CRU-219 -> CRU-220 -> CRU-221 -> (CRU-222 + CRU-223 + CRU-224) -> CRU-225 -> (CRU-226 -> CRU-227 -> CRU-228 -> CRU-229 -> CRU-230) -> CRU-231 -> CRU-232 -> (CRU-235 -> CRU-236 -> CRU-237 -> CRU-238 -> CRU-239 -> CRU-240 -> CRU-243 -> CRU-244 -> CRU-241 -> CRU-242) -> CRU-233 -> CRU-234`.
- Scope intent: add wave-army escalation, difficulty-specific enemy doctrines, boss-gated tier unlocks, and hero-tier recruitment while staying compatible with deterministic major/minor and item revamp systems.

### CRU-218 - Major/Minor Count Parity Contract (Player and Enemy Armies)
- Status: `DONE`
- Type: `Docs`
- Priority: `P0`
- Depends on: `CRU-200`
- Goal: Freeze shared upgrade-count math so enemy armies receive the same major/minor count structure as player progression.
- Context:
  - Enemy-army progression must feel fair and readable against player growth.
  - Randomness is desired in enemy loadouts, but major/minor counts must stay deterministic by level.
  - Files expected to change: `docs/SYSTEMS_REFERENCE.md`, `docs/SYSTEM_SCOPE_MAP.md`, `docs/requirements.md`.
- Implementation:
  1. Define canonical count formula: `major_count = floor(level / 5)`, `minor_count = level - major_count`.
  2. Add explicit worked examples (`level 30 => 6 major + 24 minor`; `level 100 => 20 major + 80 minor`).
  3. Define enemy-army parity rule: use same formula based on assigned army level on all difficulties.
  4. Define randomness boundaries: selection randomness allowed in options chosen, but not in count math.
- Unit Tests Required:
  - `none (docs-only task)`
  - `none (docs-only task)`
- Acceptance Criteria:
  - Player and enemy army major/minor counts are specified by one shared formula.
  - Formula examples are documented and referenced by progression and enemy-army tasks.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/requirements.md`

### CRU-219 - Difficulty Profile Contract and Match Setup Toggle
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-218`
- Goal: Add a map-setup difficulty toggle (`Recruit`, `Experienced`, `Alone Against the Infidels`) and propagate difficulty profile context to runtime systems.
- Context:
  - Army generation, scaling, AI behavior, and enemy equipment policies differ by selected difficulty.
  - Difficulty selection must be visible and persisted across run start, save/load, and analytics logging.
  - Files expected to change: `src/ui.rs`, `src/model.rs`, `src/game.rs`, `assets/data/maps.json` (or dedicated difficulty config).
- Implementation:
  1. Add difficulty selector UI to match setup and store selected value in run setup state.
  2. Add data-driven difficulty profile schema for scaling multipliers and behavior toggles.
  3. Wire selected difficulty into wave spawning, enemy-army generation, and combat AI systems.
  4. Add default/fallback handling for legacy saves or missing config.
- Unit Tests Required:
  - Setup state test verifies selected difficulty persists into run state.
  - Config validation test rejects missing/invalid difficulty profile entries.
- Acceptance Criteria:
  - Players can choose one of the three difficulties before run start.
  - Runtime systems receive and honor selected difficulty profile settings.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-220 - Wave Army Layering and Hard Wave-Lock Flow
- Status: `DONE`
- Type: `Core`
- Priority: `P0`
- Depends on: `CRU-219`
- Goal: Implement layered enemy-army cadence with hard wave lock: small armies every wave, minor armies every other wave, and boss armies every 10th wave.
- Context:
  - Next wave must never start until all enemies from the current wave are defeated.
  - Wave-10 boss fights must be decisive branch points for run progression.
  - Files expected to change: `src/enemies.rs`, `src/waves.rs` (if present), `src/model.rs`.
- Implementation:
  1. Add wave scheduler lanes for `small_army`, `minor_army`, and `major_army` events.
  2. Define spawn-volume and baseline-strength contracts for small/minor/major army categories.
  3. Enforce hard wave-lock gate so no overflow spawns into the next wave.
  4. Add deterministic seed behavior for army lane scheduling in test harness.
- Unit Tests Required:
  - Wave scheduler test verifies small/minor/major cadence across waves `1..100`.
  - Wave-lock test verifies no next-wave start until enemy count reaches zero.
- Acceptance Criteria:
  - Layered army spawns follow configured cadence exactly.
  - Wave-overflow behavior is eliminated under stress/replay tests.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-221 - Boss-Kill Tier Unlock Gate (Replace Wave-Number Unlock)
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-220`
- Goal: Replace wave-number tier unlock conditions with boss-army defeat unlocks (including hero-tier unlock path at wave `60`).
- Context:
  - Unlock progression should be earned by defeating major armies, not passive wave advancement.
  - Under hard wave-lock rules, failure state remains binary (player dies or boss dies).
  - Files expected to change: `src/squad.rs`, `src/enemies.rs`, `src/ui.rs`, `src/model.rs`.
- Implementation:
  1. Remove direct wave-number tier unlock checks from upgrade validation.
  2. Add unlock-state updates triggered by major-army defeat events (`10,20,30,40,50,60` milestones).
  3. Update upgrade UI messaging to show boss-defeat gating instead of wave-only gating.
  4. Add migration handling for runs/saves that still store wave-gated unlock metadata.
- Unit Tests Required:
  - Unlock progression test verifies tier unlock only flips after corresponding boss defeat.
  - Validation test verifies locked tier upgrades remain unavailable before boss defeat.
- Acceptance Criteria:
  - Defeating each major army unlocks the intended next tier.
  - No remaining runtime unlock behavior depends solely on wave index.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-222 - Difficulty-Aware Enemy Army Composition and Strategy Generator
- Status: `DONE`
- Type: `Core`
- Priority: `P0`
- Depends on: `CRU-220, CRU-221`
- Goal: Implement strategy-weighted enemy-army composition generation per difficulty with deterministic-random variety.
- Context:
  - `Recruit`: random composition with support cap `<= 25%`, same retinue size as player, pool spans tier `0` through the newly unlocked tier for that boss bracket.
  - `Experienced`: strategy roll biases composition; pool uses adjacent tiers (`current highest unlocked` + `tier to unlock`).
  - `Alone Against the Infidels`: advanced strategy roll controls composition + behavior; pool uses `tier to unlock` plus `Hero` tier when available.
  - Files expected to change: `src/enemies.rs`, `src/ai.rs`, `src/data.rs`, enemy army data assets.
- Implementation:
  1. Add army strategy profile definitions (`range_dominant`, `melee_dominant`, `cavalry_dominant`, and advanced variants).
  2. Implement difficulty-specific unit-pool selection and composition weighting rules.
  3. Enforce support-share caps and composition sanity checks by difficulty.
  4. Add deterministic seed-driven random choice to keep replay stability with encounter variety.
- Unit Tests Required:
  - Composition test verifies difficulty pool constraints and support cap enforcement.
  - Strategy weighting test verifies composition distributions follow strategy bias rules.
- Acceptance Criteria:
  - Enemy armies vary encounter-to-encounter without violating difficulty-specific constraints.
  - Composition generation is deterministic under identical seed + state inputs.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-223 - Enemy Army Progression Parity and Difficulty Item Loadouts
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-202, CRU-207, CRU-208, CRU-218, CRU-222`
- Goal: Apply level-based major/minor parity and difficulty-tuned randomized item loadouts to enemy armies using the same deterministic item framework as players.
- Context:
  - `Recruit` army level is `floor(player_level / 2)`; `Experienced` and `Infidels` use player level parity.
  - Enemy armies should receive random upgrade selections from major/minor pools, but counts must follow shared `level/5` formula.
  - Enemy items should be sampled from chest-equivalent item classes/templates (not legacy roll-tier stat generation).
  - Files expected to change: `src/enemies.rs`, `src/upgrades.rs`, `src/inventory.rs`, enemy army data assets.
- Implementation:
  1. Compute enemy army level by difficulty and derive major/minor counts from shared formula.
  2. Add strategy-weighted major/minor selection logic for army upgrades.
  3. Add difficulty policies for army equipment slot fill rates and item-class bands (for example recruit baseline at `1/3` filled slots).
  4. Reuse deterministic item templates and validators for enemy equipment generation.
- Unit Tests Required:
  - Progression parity test verifies enemy major/minor counts match formula at representative levels.
  - Loadout generation test verifies item assignment uses deterministic item templates and respects difficulty slot-fill rules.
- Acceptance Criteria:
  - Enemy-army progression mirrors player major/minor count structure at equivalent level rules.
  - Enemy equipment randomness remains varied while staying inside deterministic item schema constraints.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-224 - Difficulty AI Behavior Layers for Army Units
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P1`
- Depends on: `CRU-222`
- Goal: Implement incremental AI behavior stacks by difficulty, including dodge, block usage, spacing, and strategy-linked behavior.
- Context:
  - `Recruit`: no block and no ranged-dodge behavior.
  - `Experienced`: higher stat scaling, ranged-dodge enabled, block enabled, and composition-linked active skill usage.
  - `Infidels`: stronger scaling, ranged/support melee-avoidance, broader strategy behavior archetypes (`aggressive`, `defensive`, `hit_and_run`).
  - Files expected to change: `src/ai.rs`, `src/combat.rs`, `src/enemies.rs`.
- Implementation:
  1. Add difficulty gates for dodge/block behavior toggles and tuning values.
  2. Implement ranged/support spacing logic for high difficulty melee avoidance.
  3. Bind strategy profiles to behavior packages and active-skill usage weighting.
  4. Add fallback behavior guards to avoid AI deadlocks/stutter loops.
- Unit Tests Required:
  - Behavior toggle tests verify difficulty-specific block/dodge activation.
  - Spacing behavior tests verify ranged/support units maintain separation on high difficulty.
- Acceptance Criteria:
  - Difficulty tiers produce visibly different combat behavior, not only stat scaling.
  - AI remains stable under dense battles and mixed-unit compositions.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-225 - Boss Army Reward Contract (Dual Chest Drops)
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P1`
- Depends on: `CRU-220, CRU-223`
- Goal: Make defeated major armies drop two equipment chests at nearby distinct positions.
- Context:
  - Boss rewards should feel meaningfully larger than normal wave clear rewards.
  - Chest drop placement must avoid overlap/clipping and remain collectible under clutter.
  - Files expected to change: `src/drops.rs`, `src/enemies.rs`, `src/visuals.rs`.
- Implementation:
  1. Add major-army defeat reward event path that spawns exactly two chests.
  2. Implement positional spread logic for chest spawn offsets relative to boss death location.
  3. Ensure chest generation uses current deterministic item/chest pipeline.
  4. Add anti-duplication guards so reward event cannot fire twice for same boss instance.
- Unit Tests Required:
  - Boss reward test verifies two chest entities spawn once per major-army defeat.
  - Chest spread test verifies generated spawn offsets are non-overlapping and in valid bounds.
- Acceptance Criteria:
  - Every major-army defeat yields two collectible chests.
  - No duplicate chest exploit occurs from replayed/late events.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-226 - Unit Tier Tree Naming and Branch Contract
- Status: `DONE`
- Type: `Docs`
- Priority: `P0`
- Depends on: `CRU-221`
- Goal: Finalize canonical unit names/branches for tiers `1..5`, including any renames that better match role descriptions, before phased implementation.
- Context:
  - Current proposed tree is broad and includes role-divergent branches (frontline, anti-cavalry, glass-cannon, summon/scout, support variants).
  - Naming must remain readable and faction-compatible while preserving branch identity.
  - Files expected to change: `docs/SYSTEMS_REFERENCE.md`, `docs/SYSTEM_SCOPE_MAP.md`, `assets/data/units.json` (or equivalent).
- Implementation:
  1. Freeze tier-branch map for infantry/ranged/support lines and confirm tier transitions `T1 -> T5`.
  2. Finalize renamed display names and stable internal IDs for each branch node.
  3. Define role tags and trait descriptors used by stat-band UI and tooltips.
  4. Define branch-level exclusions and prerequisites used in promotion UI.
- Unit Tests Required:
  - `none (design/docs task)`
  - `none (design/docs task)`
- Acceptance Criteria:
  - Tier tree and naming map are implementation-ready with no unresolved branch ambiguity.
  - Every node has role tags and descriptor text compatible with qualitative stat UI.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-227 - Tier 1 Roster Implementation
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-226`
- Goal: Implement tier-1 unit roster unlock and promotion paths (frontline infantry, ranged bowman, support priest line).
- Context:
  - Tier-1 should establish first meaningful specialization above tier-0 baseline.
  - Boss wave `10` defeat unlock must gate these promotions.
  - Files expected to change: unit data assets, `src/squad.rs`, `src/combat.rs`, `src/ui.rs`.
- Implementation:
  1. Add tier-1 unit definitions with role tags and qualitative descriptor metadata.
  2. Wire promotion validation and stat/effect initialization for tier-1 paths.
  3. Update unit-upgrade UI to surface tier-1 nodes as distinct selectable boxes per source unit row.
  4. Add initial balancing pass for tier-1 role differentiation.
- Unit Tests Required:
  - Promotion path test verifies only valid tier-1 upgrades are selectable after unlock.
  - Combat behavior test verifies tier-1 units apply their intended role traits.
- Acceptance Criteria:
  - Tier-1 upgrades are fully playable and gated by boss-defeat unlock state.
  - Tier-1 branches produce clearly distinct battlefield roles.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-228 - Tier 2 Roster Implementation (Branch Expansion + Scout/Tracker Behaviors)
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-227`
- Goal: Implement tier-2 branch expansion, including scout/tracker autonomous attack behaviors and support branch divergence.
- Context:
  - Tier-2 introduces multi-branch choices per line and the first high-variance behavior kits.
  - Scout/tracker mechanics add temporary off-formation action and summon-like behavior.
  - Files expected to change: unit data assets, `src/squad.rs`, `src/combat.rs`, `src/ai.rs`, `src/ui.rs`.
- Implementation:
  1. Add tier-2 branch node definitions and promotion graph links.
  2. Implement tracker timed hound behavior and scout temporary out-of-formation strike behavior.
  3. Implement support branch constraints (for example armor immunity/no-armor progression where specified).
  4. Add UI/state handling to keep formation accounting correct while temporary behavior is active.
- Unit Tests Required:
  - Tracker/scout behavior tests verify timer cadence, duration, and return-to-formation behavior.
  - Promotion graph test verifies all tier-2 branch paths are valid and no illegal cross-links exist.
- Acceptance Criteria:
  - Tier-2 branches are functional and strategically distinct.
  - Temporary autonomous behaviors do not break retinue counts or formation integrity.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-229 - Tier 3 Roster Implementation
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P1`
- Depends on: `CRU-228`
- Goal: Implement tier-3 upgrades for all active branches with consistent role identity and progression scaling.
- Context:
  - Tier-3 should feel like role consolidation (sharper identities, stronger tradeoffs).
  - All existing tier-2 branches must have a valid tier-3 continuation.
  - Files expected to change: unit data assets, `src/squad.rs`, `src/combat.rs`, `src/ui.rs`.
- Implementation:
  1. Add tier-3 unit nodes for all supported branch lines.
  2. Tune role signatures and trait interactions for strategic contrast.
  3. Update promotion UI rendering and validation for tier-3 visibility and gating.
  4. Add branch regression tests for tier-3 upgrade correctness.
- Unit Tests Required:
  - Tier-3 promotion path test verifies each tier-2 node maps to expected tier-3 node.
  - Trait regression test verifies tier-3 upgrades preserve intended role mechanics.
- Acceptance Criteria:
  - Every active branch has a working tier-3 continuation.
  - Tier-3 upgrades create meaningful power/role progression without branch collapse.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-230 - Tier 4 Roster Implementation
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P1`
- Depends on: `CRU-229`
- Goal: Implement tier-4 upgrades across all branches and rebalance high-tier role tradeoffs.
- Context:
  - Tier-4 introduces near-endgame branch identity and must not erase weaknesses.
  - Branch readability in upgrade UI must remain clear as option count grows.
  - Files expected to change: unit data assets, `src/squad.rs`, `src/combat.rs`, `src/ui.rs`.
- Implementation:
  1. Add tier-4 node definitions and stat/trait deltas for each branch.
  2. Tune endgame-role tradeoffs to prevent one-branch dominance.
  3. Update upgrade UI layout/performance for increased node count.
  4. Add high-tier promotion and combat regression tests.
- Unit Tests Required:
  - Tier-4 promotion validation tests for all branch paths.
  - Endgame combat sanity test verifies no invalid stat/state initialization at tier-4.
- Acceptance Criteria:
  - Tier-4 promotions are stable and available for all intended branches.
  - High-tier branch choice remains strategic rather than solved.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-231 - Tier 5 + Hero Tier Unlock Scaffold and Hear the Call Resource
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-230, CRU-220, CRU-221`
- Goal: Implement tier-5 branch capstones, hero-tier unlock at wave-60 boss defeat, and `Hear the Call` token economy (`1 token = 1 hero`).
- Context:
  - Hero tier is gated behind late progression and should not appear before wave-60 boss defeat.
  - `Hear the Call` must be an explicit resource surfaced anywhere hero recruitment is possible.
  - Files expected to change: `src/drops.rs`, `src/model.rs`, `src/squad.rs`, `src/ui.rs`, resource/data assets.
- Implementation:
  1. Implement tier-5 unit nodes and final branch caps for all lines.
  2. Add wave-60 boss defeat trigger for hero-tier unlock state.
  3. Add `Hear the Call` drop/resource tracking and persistence.
  4. Enforce token consumption (`1 token` spent per hero recruitment action).
- Unit Tests Required:
  - Token economy test verifies hero recruitment consumes exactly one token.
  - Unlock gate test verifies hero tier remains locked before wave-60 boss defeat.
- Acceptance Criteria:
  - Tier-5 branches are playable and fully connected from lower tiers.
  - Hero tier unlock and `Hear the Call` resource flow work end-to-end.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-232 - Unit Upgrade UI Expansion: Horizontal Subtype Boxes + Hero Recruit Actions
- Status: `DONE`
- Type: `UI`
- Priority: `P0`
- Depends on: `CRU-231, CRU-226`
- Goal: Expand the unit upgrade screen to show subtype boxes horizontally per tier/unit type and enable direct hero recruitment from hero boxes.
- Context:
  - Infantry/ranged/support/cavalry hero subtype choices must be visible as separate selectable boxes.
  - Hero box click should recruit if requirements are met; otherwise show disabled reason states.
  - Files expected to change: `src/ui.rs`, `src/squad.rs`, `src/model.rs`, `src/visuals.rs`.
- Implementation:
  1. Add horizontal subtype box rows for each tier/type branch group in the upgrade modal.
  2. Add hero subtype row with click-to-recruit actions tied to `wave >= 60` unlock and `Hear the Call` availability.
  3. Reuse existing double-click/equip interaction safety patterns for click actions and confirmations where needed.
  4. Add clear disabled-state reasons (`Requires wave 60 boss unlock`, `Requires Hear the Call`).
- Unit Tests Required:
  - UI interaction test verifies hero recruit action triggers only when requirements are met.
  - Disabled-state test verifies correct reason text for each unmet requirement.
- Acceptance Criteria:
  - Upgrade screen displays subtype options as separate horizontal boxes in each relevant tier lane.
  - Hero recruitment can be performed directly from hero boxes with correct gating and token checks.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-235 - Faction-Agnostic Identity Contract (UnitRef/HeroRef/ItemRef Data Ownership)
- Status: `DONE`
- Type: `Docs`
- Priority: `P0`
- Depends on: `CRU-232`
- Goal: Freeze a canonical identity model where runtime systems use generic unit/hero/item IDs with faction context instead of faction-duplicated definitions.
- Context:
  - Current runtime still carries faction-duplicated identifiers (`Christian*`, `Muslim*`) across combat, UI, promotion, and archive paths.
  - The refactor target is config-extensible onboarding of new factions without re-wiring core systems.
  - Files expected to change: `docs/SYSTEMS_REFERENCE.md`, `docs/SYSTEM_SCOPE_MAP.md`, `docs/requirements.md`.
- Implementation:
  1. Define canonical runtime identity structs (`UnitRef { faction_id, unit_id }`, `HeroRef { faction_id, hero_id }`, `ItemRef { faction_id, item_id }`) and naming conventions.
  2. Define ownership boundaries for base data vs faction override data (`display`, `stats`, `abilities`, `visuals`, `promotion`, `hero pools`, `item drop tables`, `item icons`).
  3. Define merge/fallback contract for unresolved override fields and validation failures.
  4. Define migration guardrails and explicit cutover criteria (no backwards compatibility layer after cutover).
- Unit Tests Required:
  - `none (design/docs task)`
  - `none (design/docs task)`
- Acceptance Criteria:
  - One approved identity contract exists for units/heroes/items that is faction-agnostic.
  - Core/runtime/content tasks can implement against this contract without unresolved ambiguity.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/requirements.md`

### CRU-236 - Data Schema Migration: Generic Unit/Hero/Item IDs + Faction Overrides
- Status: `DONE`
- Type: `Core`
- Priority: `P0`
- Depends on: `CRU-235`
- Goal: Replace faction-duplicated content schema with base catalogs plus faction override schema for units, heroes, and items.
- Context:
  - Current JSON/data and enum wiring duplicates per-faction roster entries.
  - New schema must support adding a new faction through data files without gameplay-code branch explosion.
  - Files expected to change: `src/data.rs`, `assets/data/units*.json`, `assets/data/heroes*.json`, `assets/data/items*.json`, drop-table configs, validators.
- Implementation:
  1. Add generic base catalogs (`unit_id`, `hero_id`, `hero_subtype_id`, `item_id`) and faction override sections keyed by generic IDs.
  2. Add data loader merge logic (base -> faction override) with strict validation and clear error reporting.
  3. Migrate existing Christian/Muslim data into override files while preserving current behavior parity.
  4. Reject legacy faction-duplicated schema forms after migration cutover.
- Unit Tests Required:
  - Loader/validator tests for base+override merge success and invalid override failure.
  - Migration parity tests verifying representative unit/hero/item entries preserve expected resolved runtime values.
- Acceptance Criteria:
  - Runtime loads generic unit/hero/item catalogs with faction overrides only.
  - Invalid faction overrides fail fast with actionable validation messages.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-237 - Runtime Identity Refactor Across ECS (UnitKind -> UnitRef)
- Status: `DONE`
- Type: `Core`
- Priority: `P0`
- Depends on: `CRU-236`
- Goal: Refactor runtime entity identity and decision logic to consume generic `UnitRef`/`HeroRef` instead of faction-specific unit enums.
- Context:
  - Systems currently key behavior through long `match` blocks on faction-specific `UnitKind` variants.
  - This card is the core mechanical cutover and must maintain deterministic gameplay behavior.
  - Files expected to change: `src/model.rs`, `src/squad.rs`, `src/combat.rs`, `src/enemies.rs`, `src/inventory.rs`, `src/formation.rs`, `src/morale.rs`.
- Implementation:
  1. Introduce `UnitRef`/`HeroRef` runtime types and replace entity identity usage in combat/squad/enemy/formation loops.
  2. Replace faction-specific trait checks (ranged/support/tracker/scout/etc.) with resolved catalog traits/tags.
  3. Replace label/stat/profile lookup helpers with catalog-driven resolvers.
  4. Remove dead branch logic that depended on faction-specific enum names.
- Unit Tests Required:
  - Identity resolution tests covering role/trait lookup parity for representative units.
  - Deterministic regression tests for combat/squad behavior equivalence under fixed seeds.
- Acceptance Criteria:
  - Core runtime no longer requires faction-specific unit enum variants to resolve behavior.
  - Existing runs remain behaviorally stable under deterministic replay checks.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-238 - Generic Promotion/Rescue/Wave Pipeline Refactor
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-237`
- Goal: Rebuild promotion trees, rescue recruitment, and wave composition on generic IDs with faction-scoped content resolution.
- Context:
  - Promotion graph and rescue/wave paths still rely on faction-specific unit naming and branch duplication.
  - The target is one generic progression graph with faction-level content overrides where needed.
  - Files expected to change: `src/squad.rs`, `src/rescue.rs`, `src/enemies.rs`, roster/promotion data assets.
- Implementation:
  1. Move promotion graph definitions to generic IDs with faction override support for branches or exclusions.
  2. Refactor rescue pool generation to select generic IDs plus faction context.
  3. Implement wave-tier composition ramp:
     - waves `1..10` spawn `100%` tier-0 units,
     - after each major-army unlock, regular waves blend previous/current unlocked tiers so current-tier share ramps to `100%` by the next major wave (`20/30/40/50/60`),
     - major-army lane includes a difficulty-scaled preview slice of the next tier (when available).
  4. Keep boss-gated unlock and economy constraints intact under generic IDs.
  5. Add migration checks ensuring no orphaned promotion edges or invalid pool references.
- Unit Tests Required:
  - Promotion graph integrity tests (no illegal edges, no orphan nodes) across all factions.
  - Rescue/wave selection tests verifying faction-correct resolution from generic IDs.
  - Wave-tier mix tests verifying ramp milestones (`wave 11`, `wave 20`, `wave 21`) and major-wave preview behavior by difficulty.
- Acceptance Criteria:
  - Promotion/rescue/wave systems operate from one generic content model.
  - Regular-wave tier composition follows unlock-ramp contract and reaches `100%` unlocked-tier composition by the next major wave.
  - Major-army waves include a measurable next-tier preview share when a higher tier exists.
  - Faction differences are data overrides, not duplicated system logic.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-239 - UI/Archive Naming Refactor (Generic Names + Faction Presentation Layer)
- Status: `DONE`
- Type: `UI`
- Priority: `P1`
- Depends on: `CRU-237, CRU-238`
- Goal: Remove hardcoded faction prefixes from unit/hero/item display paths and route naming through resolved catalog display data.
- Context:
  - UI currently displays names through hardcoded label maps (`Christian ...`, `Muslim ...`).
  - Design intent is reusable generic names with faction-specific presentation via icons/colors/optional override names.
  - Files expected to change: `src/ui.rs`, `src/archive.rs`, `src/inventory.rs`, related data assets.
- Implementation:
  1. Replace hardcoded label maps in UI/archive with catalog display-name resolvers.
  2. Add faction presentation metadata (badge/icon/tint/optional display override) without duplicating unit definitions.
  3. Update tooltips, upgrade graphs, inventory/chest item cards, and archive entries to use resolved names.
  4. Remove stale tests that rely on hardcoded faction-prefixed labels and replace with resolver-based assertions.
- Unit Tests Required:
  - UI/archive/inventory name resolution tests for generic names with faction presentation tags.
  - Tooltip regression tests verifying role/stats/ability sections remain intact after resolver cutover.
- Acceptance Criteria:
  - Display naming is generic-first and faction-aware through presentation metadata.
  - No hardcoded faction-prefixed unit labels remain in runtime UI code paths.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-240 - Hero System Genericization (Subtype Pools + Faction Overrides)
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-236, CRU-237, CRU-238`
- Goal: Convert hero recruitment and hero content to generic hero IDs/subtypes with faction override pools and data-driven rolls.
- Context:
  - Current hero recruit scaffolding maps subtype clicks to faction-specific placeholder unit kinds.
  - This card prepares robust hero architecture so content tickets can become pure data authoring.
  - Files expected to change: `src/squad.rs`, `src/data.rs`, `src/ui.rs`, hero data assets.
- Implementation:
  1. Define generic hero subtype pools and recruit resolver using `FactionId + HeroSubtypeId`.
  2. Add faction override hooks for hero stats/abilities/display names per hero entry.
  3. Replace placeholder subtype-to-unit mapping with data-resolved hero selection.
  4. Keep token spend and wave-60 unlock gates unchanged.
- Unit Tests Required:
  - Hero recruit resolver tests for subtype/faction correctness and duplicate-allowed policy.
  - Runtime recruit tests ensuring token/unlock gating still works after generic hero cutover.
- Acceptance Criteria:
  - Hero recruitment is fully data-driven on generic IDs.
  - Adding a new faction hero pool requires data changes only.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-243 - Item Catalog + Drop Table Genericization (ItemRef + Faction Overrides)
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-236, CRU-237`
- Goal: Convert item definitions and drop tables to generic `ItemRef` with faction-specific overrides for item behavior, visuals, and availability.
- Context:
  - Current item/drop behavior still assumes fixed two-faction wiring in several runtime paths.
  - We need new factions to onboard through data-only item/drop configuration.
  - Files expected to change: `src/data.rs`, `src/inventory.rs`, `src/drops.rs`, `assets/data/items*.json`, `assets/data/drops*.json`.
- Implementation:
  1. Add generic item catalog IDs and faction override sections for stats/effects/availability rules.
  2. Add faction-aware drop-table schema for ambient packs, enemy drops, and chest pools.
  3. Add merge and validation rules so unresolved faction item references fail fast.
  4. Migrate current Christian/Muslim item/drop behavior into override data with parity checks.
- Unit Tests Required:
  - Item/drop schema validation tests (valid/invalid faction override references).
  - Drop table parity tests verifying existing faction behavior remains unchanged after migration.
- Acceptance Criteria:
  - Item definitions and drop pools resolve through `ItemRef + faction_id`.
  - New faction item/drop onboarding is data-only with no new gameplay branch code.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-244 - Item Icon/Presentation Resolver + Runtime Drop Integration
- Status: `DONE`
- Type: `UI`
- Priority: `P1`
- Depends on: `CRU-239, CRU-243`
- Goal: Route inventory/chest/drop icon selection and item presentation through faction-aware data resolvers instead of hardcoded icon picks.
- Context:
  - Some item icons already vary by faction (for example symbol/magnet assets), but this needs to be fully data-driven and generalized.
  - Visual and drop presentation parity must hold for existing factions after cutover.
  - Files expected to change: `src/ui.rs`, `src/inventory.rs`, `src/drops.rs`, icon mapping data assets.
- Implementation:
  1. Replace hardcoded icon-selection branches with resolver lookup (`item_id + faction_id`).
  2. Apply resolver path to inventory slots, chest cards, drop pickups, and tooltips.
  3. Add fallback icon policy and explicit validation for missing faction icon mappings.
  4. Add fixture coverage for faction-specific icon swaps and drop-surface consistency.
- Unit Tests Required:
  - Icon resolver tests for faction-specific and fallback icon selection.
  - UI/runtime tests confirming chest/drop/inventory surfaces use resolved icon mappings.
- Acceptance Criteria:
  - Item icon/presentation differences are fully data-driven by faction override config.
  - No hardcoded faction-specific item icon branches remain in runtime UI logic.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-245 - Hear the Call Chest Drop Refactor (All Waves, Lane-Scaled RNG)
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-238, CRU-244`
- Goal: Move `Hear the Call` into chest-based pickups and source drops from all enemy lanes with very low, wave-scaled probability.
- Context:
  - Current token flow is deterministic major-wave gated and uses direct pickup behavior.
  - Design update requires tiny RNG drops from all waves/enemies (`small`, `minor`, `major`) with gentle wave scaling and anti-hoard controls.
  - Existing periodic random world chest cadence must remain intact (every 3 waves).
  - Files expected to change: `src/drops.rs`, `src/combat.rs`, `src/enemies.rs`, `src/model.rs`.
- Implementation:
  1. Add enemy death metadata so drop logic can distinguish lane size (`small`, `minor`, `major`) at runtime.
  2. Add lane-specific base/cap probabilities with slight wave scaling and stash-damping based on unspent tokens.
  3. Trigger `Hear the Call` chest spawn rolls on enemy deaths across all waves.
  4. Remove direct deterministic token spawn assumptions from major-wave handlers.
- Unit Tests Required:
  - Drop chance contract test verifies lane ordering (`major > minor > small`), wave scaling, and stash damping.
  - Death metadata propagation test verifies enemy lane source survives into drop-eligible death events.
- Acceptance Criteria:
  - `Hear the Call` can drop before wave 60 with very low chance and scales modestly by lane/wave.
  - Higher-size lanes are measurably more likely than small lane, while stash damping reduces hoarding.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-246 - Hear the Call Single-Item Chest Pickup Flow
- Status: `DONE`
- Type: `UI`
- Priority: `P1`
- Depends on: `CRU-245`
- Goal: Ensure `Hear the Call` is collected via dedicated chest interaction (single-item reward), not auto-pickup.
- Context:
  - Player request is chest-based token collection with intentional pickup time, distinct from ambient gold behavior.
  - `Hear the Call` chest should grant exactly one token and should not open the equipment chest inventory modal.
  - Files expected to change: `src/drops.rs`, optional HUD/tooltip surfaces in `src/ui.rs`.
- Implementation:
  1. Convert `Hear the Call` world drop entity to chest-style presentation and channel pickup behavior.
  2. Keep token chest reward payload single-item (`+1 Hear the Call`) and immediate consume on completion.
  3. Preserve equipment chest flow for item rewards without cross-contaminating chest payload logic.
  4. Add/adjust visual/readability hints so token chests are distinguishable from item chests.
- Unit Tests Required:
  - Pickup timing test verifies token chest requires channel completion and is not auto-collected.
  - Reward payload test verifies one chest grants exactly one token and no equipment modal opening.
- Acceptance Criteria:
  - `Hear the Call` drops are picked up through chest interaction and cannot be vacuumed like gold packs.
  - Token chests grant only token currency and do not inject equipment chest state.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-247 - Unit Role Counter Matrix Contract (Core + Heroes)
- Status: `DONE`
- Type: `Docs`
- Priority: `P0`
- Depends on: `CRU-200, CRU-238`
- Goal: Define an explicit strengths/weaknesses contract for every unit line (including heroes) so balancing and UI can align on one counter matrix.
- Context:
  - Current roster has role identity hints, but no single formal matrix defining counter relationships and tradeoffs per line.
  - Player direction requires clear specialization (for example: shield infantry as meat shields, spearmen anti-cavalry, cavalry anti-infantry, crossbows anti-armor, hero subtype parallels like javelin hero as anti-armor).
  - Files expected to change: `docs/SYSTEMS_REFERENCE.md`, `docs/SYSTEM_SCOPE_MAP.md`, `docs/requirements.md`.
- Implementation:
  1. Define canonical role tags and armor classes (for example: `frontline`, `anti_cavalry`, `anti_armor`, `skirmisher`, `support`, `hybrid`, `hero_doctrine`).
  2. Author a full counter matrix for tier lines and hero subtypes (offense strengths, defensive weaknesses, and notable utility traits).
  3. Define tradeoff rules per specialization so strengths always come with a weakness or opportunity cost.
  4. Define data contract for storing these traits and counter hooks in faction-agnostic catalogs.
- Unit Tests Required:
  - `none (docs/design contract task)`
  - `none (docs/design contract task)`
- Acceptance Criteria:
  - One approved matrix exists covering all unit lines and all hero subtypes.
  - Counter relationships and tradeoffs are unambiguous and implementation-ready.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/requirements.md`

### CRU-248 - Counter Mechanics Runtime Integration (Tag-Driven Combat Hooks)
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-247, CRU-237`
- Goal: Implement runtime combat hooks that apply the defined strengths/weaknesses through tag-driven multipliers and behavior rules.
- Context:
  - Counter behavior should be data-driven and faction-agnostic, not hardcoded branch lists.
  - Must integrate with major/minor upgrade and item revamp so upgrades can shift or amplify counters.
  - Files expected to change: `src/combat.rs`, `src/data.rs`, `src/model.rs`, `src/enemies.rs`, unit/hero data assets.
- Implementation:
  1. Add runtime-resolved trait/tag payloads for attacker and defender profiles (including armor class and role counters).
  2. Apply counter hooks in damage and mitigation paths (for example anti-armor vs armored targets, anti-cavalry vs cavalry targets).
  3. Add hooks for counter-aware utility interactions (block/dodge/formation pressure where applicable).
  4. Keep all effects bounded and deterministic with explicit caps and clamp rules.
- Unit Tests Required:
  - Counter resolution tests verifying expected matchup outcomes (for example spear vs cavalry, crossbow/javelin vs armor, cavalry vs infantry).
  - Determinism tests verifying counter hooks remain stable under fixed seeds and replay conditions.
- Acceptance Criteria:
  - Core combat consistently applies role counters from data tags.
  - No matchup effect requires faction-specific hardcoded logic branches.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-249 - Hero Subtype Specialization Pass (Strength/Weakness Parity)
- Status: `DONE`
- Type: `Balance`
- Priority: `P0`
- Depends on: `CRU-247, CRU-248, CRU-240`
- Goal: Specialize hero subtypes to mirror roster doctrine patterns with stronger upside and explicit weakness profiles.
- Context:
  - Heroes are currently scaffolded around subtype recruit actions, but detailed combat identities are not finalized.
  - Player direction requires subtype-specific doctrine clarity (for example javelin hero as premium anti-armor).
  - Files expected to change: hero data assets, `src/squad.rs`, `src/combat.rs`, `src/ui.rs`.
- Implementation:
  1. Define hero subtype stat/ability packages aligned with the counter matrix.
  2. Add explicit subtype weaknesses/tradeoffs to prevent all-purpose hero dominance.
  3. Integrate subtype behavior with upgrade/item interactions and ensure no contradictory stacking.
  4. Tune subtype power ceilings so heroes remain strategic picks, not automatic best choices.
- Unit Tests Required:
  - Hero subtype profile tests verifying each subtype exposes expected doctrine tags and weaknesses.
  - Matchup tests confirming hero subtype counters/penalties apply correctly in runtime combat.
- Acceptance Criteria:
  - Every hero subtype has a distinct role and counter profile.
  - Hero subtype performance follows design intent without collapsing into one dominant pick.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-250 - Counter Clarity UI Pass (Strengths/Weaknesses and Trait-First Tooltips)
- Status: `DONE`
- Type: `UI`
- Priority: `P1`
- Depends on: `CRU-247, CRU-248, CRU-249, CRU-216`
- Goal: Expose strengths/weaknesses clearly in unit/hero UI surfaces using trait-first language and stat-band presentation.
- Context:
  - Player-facing clarity is required so strategic counter choices are readable without relying on raw hidden numbers.
  - Must align with qualitative band UX direction and avoid contradictory tooltip language.
  - Files expected to change: `src/ui.rs`, archive UI paths, tooltip render helpers, documentation.
- Implementation:
  1. Add standardized tooltip sections for `Strengths`, `Weaknesses`, and `Key Matchups`.
  2. Map runtime trait tags to clear player-facing descriptors and icons/labels.
  3. Ensure hero subtype selection UI and unit-upgrade graph surfaces show doctrine differences.
  4. Add fallback behavior for incomplete trait metadata with validation warnings.
- Unit Tests Required:
  - Tooltip/render tests verifying strengths/weakness sections appear with valid trait mappings.
  - UI regression tests ensuring trait-first labels remain consistent across roster, hero, and archive surfaces.
- Acceptance Criteria:
  - Players can identify unit/hero strengths and weaknesses directly from UI without inspecting internal numbers.
  - Trait presentation is consistent across all relevant screens.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-251 - Formation Role Lane Policy Contract (Tiered Units + Heroes)
- Status: `DONE`
- Type: `Docs`
- Priority: `P0`
- Depends on: `CRU-247`
- Goal: Define a canonical formation lane policy (`outer`, `middle`, `inner`) for all unit lines and hero subtypes, including fallback rules.
- Context:
  - Current slot placement is a static `UnitKind` match table and may drift from evolved tier/hero roles.
  - Player direction requires explicit per-line lane intent (for example zealot/flagellant frontline, tracker middle, support inner).
  - Files expected to change: `docs/SYSTEMS_REFERENCE.md`, `docs/requirements.md`, `docs/SYSTEM_SCOPE_MAP.md`.
- Implementation:
  1. Author lane intent per unit line and hero subtype (primary lane + secondary fallback lane).
  2. Define lane quotas and fallback behavior for mixed rosters (for example insufficient frontline population).
  3. Define formation-mode nuances (`Square` vs `Diamond`) where lane behavior should differ.
  4. Define validation invariants (no support-only line can default to outer lane without explicit exception).
- Unit Tests Required:
  - `none (docs/policy task)`
  - `none (docs/policy task)`
- Acceptance Criteria:
  - One approved lane policy covers all tiers and hero subtypes.
  - Runtime tasks can implement lane logic without ambiguity.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/requirements.md`

### CRU-252 - Formation Slot Resolver Refactor (Tag-Driven Lanes + Quotas)
- Status: `DONE`
- Type: `Core`
- Priority: `P0`
- Depends on: `CRU-251, CRU-237`
- Goal: Replace hardcoded `UnitKind` lane matching with data/tag-driven lane resolver plus quota-based ring assignment.
- Context:
  - Current logic in `formation_slot_role_priority` is manually enumerated and brittle under roster growth.
  - Refactor should align with generic identity migration and role-tag based combat architecture.
  - Files expected to change: `src/formation.rs`, `src/model.rs`, `src/data.rs`, roster metadata assets.
- Implementation:
  1. Add lane metadata/tags to resolved unit/hero profiles (`preferred_lane`, `fallback_lane`, `lane_weight`).
  2. Replace static match table with resolver-driven assignment.
  3. Add quota-aware assignment (reserve outer lanes for frontline first, cap support crowding in outer ring).
  4. Keep deterministic ordering under equal priority using stable tie-breakers.
- Unit Tests Required:
  - Resolver tests for representative units/heroes verify lane mapping and fallback behavior.
  - Deterministic slot assignment tests for mixed compositions verify quota and stable ordering rules.
- Acceptance Criteria:
  - Formation lane assignment no longer depends on hardcoded faction-specific unit lists.
  - Mixed-tier/hero rosters place units according to policy and quotas consistently.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-253 - Formation Placement Balance Pass (All Tiers + Hero Subtypes)
- Status: `DONE`
- Type: `Balance`
- Priority: `P1`
- Depends on: `CRU-252, CRU-249`
- Goal: Validate and retune lane placement outcomes for all promoted lines and hero subtypes under realistic roster compositions.
- Context:
  - Even correct policy can create poor practical outcomes without composition-level tuning.
  - Need to test edge mixes (support-heavy, ranged-heavy, cavalry-heavy, zealot-heavy, hero-heavy).
  - Files expected to change: formation tuning data and/or resolver constants, test fixtures.
- Implementation:
  1. Build representative roster fixtures for each doctrine and hero mix.
  2. Evaluate lane occupancy, survivability, and damage uptime across fixtures.
  3. Tune lane weights/fallbacks where outcomes violate intended strengths/weaknesses.
  4. Lock resulting defaults into data/config with clear commentary.
- Unit Tests Required:
  - Fixture tests asserting expected lane occupancy envelopes by doctrine composition.
  - Regression tests ensuring key identities hold (for example trackers middle, priests inner, zealot line outer).
- Acceptance Criteria:
  - Formation placement is tactically coherent across tier progression and hero recruitment.
  - No major line is consistently stranded in counterproductive lanes.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-254 - Formation Debug Surface (Lane Assignment Inspector)
- Status: `DONE`
- Type: `UI`
- Priority: `P2`
- Depends on: `CRU-252`
- Goal: Add a lightweight debug/QA overlay to inspect per-unit lane assignment in live runs.
- Context:
  - Lane resolver regressions are hard to diagnose without runtime visibility.
  - A temporary/debug-gated overlay accelerates balancing and prevents hidden placement drift.
  - Files expected to change: `src/ui.rs`, optional debug flag/config.
- Implementation:
  1. Add debug-gated overlay labels/colors for current lane assignment on friendly units.
  2. Surface lane totals per ring in HUD/debug panel.
  3. Add toggle hotkey and ensure disabled in production defaults.
  4. Document QA usage steps.
- Unit Tests Required:
  - UI state test verifies debug lane overlay toggles on/off correctly.
  - Assignment-summary test verifies displayed lane counts match resolver output.
- Acceptance Criteria:
  - QA can inspect lane assignments live without code instrumentation.
  - Overlay does not affect gameplay state when disabled.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-255 - Formation Roster Expansion Contract (Circle / Skean / Diamond Rework / Shield Wall / Loose)
- Status: `DONE`
- Type: `Docs`
- Priority: `P0`
- Depends on: `CRU-251`
- Goal: Freeze the expanded formation roster rules, effects, and constraints before implementation.
- Context:
  - Requested formation set: `Circle`, `Skean/Crescent`, reworked `Diamond`, `Shield Wall`, and `Loose`.
  - Current runtime ships `Square` and `Diamond` only, with no per-formation anti-entry/reflect/block policy matrix.
  - Files expected to change: `docs/SYSTEMS_REFERENCE.md`, `docs/SYSTEM_SCOPE_MAP.md`, `docs/requirements.md`.
- Implementation:
  1. Define exact per-formation modifiers (offense/defense/move speed/inside-entry rules/reflect/block hooks).
  2. Define hard caps and exclusions to prevent stacking abuse (for example reflect + high block + anti-entry).
  3. Define per-formation intended counters and failure states (what each formation is weak against).
  4. Define compatibility with morale, banner penalties, and movement-state bonuses.
- Unit Tests Required:
  - `none (design/docs task)`
  - `none (design/docs task)`
- Acceptance Criteria:
  - Each formation has explicit strengths, weaknesses, and bounded effect ranges.
  - Runtime implementation can proceed without unresolved spec ambiguity.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/requirements.md`

### CRU-256 - Formation Mechanics Runtime Implementation (Anti-Entry, Reflect, Loose Spread)
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-255, CRU-252`
- Goal: Implement the new formation mechanics and reworked Diamond behavior in runtime systems.
- Context:
  - Needs core changes across formation placement, collision/inside-formation checks, combat hooks, and movement modifiers.
  - Must remain deterministic and compatible with existing wave lock/combat loops.
  - Files expected to change: `src/formation.rs`, `src/combat.rs`, `src/collision.rs`, `src/squad.rs`, `src/model.rs`, formation data assets.
- Implementation:
  1. Add formation enum/data/config entries for `Circle`, `Skean`, `Diamond` (rework), `ShieldWall`, and `Loose`.
  2. Implement anti-entry enforcement for `Diamond` and `ShieldWall` (blocking enemy interior occupancy).
  3. Implement `ShieldWall` mechanics: reduced movement, block bonus for block-capable units, melee-hit reflect.
  4. Implement `Loose` mechanics: widened spacing and unlimited enemy interior occupancy.
  5. Implement `Skean` and `Circle` modifiers with movement-state and armor interactions.
- Unit Tests Required:
  - Formation rule tests for anti-entry, loose interior allowance, and spacing behavior by formation type.
  - Combat hook tests for shield-wall reflect and block bonus gating to block-capable lines only.
- Acceptance Criteria:
  - New formation mechanics are live and deterministic under replay.
  - Reworked Diamond/Skean behavior matches approved design contract.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-257 - Formation-Specific Lane Policies and Reassignment Rules
- Status: `DONE`
- Type: `Core`
- Priority: `P0`
- Depends on: `CRU-251, CRU-255, CRU-252`
- Goal: Support different unit placement policies per formation profile.
- Context:
  - Some formations need different lane behavior (for example Loose may tolerate deeper ranged spread, Shield Wall should strongly prioritize frontline shell).
  - Current slot ordering applies one global policy.
  - Files expected to change: `src/formation.rs`, lane policy metadata, docs.
- Implementation:
  1. Add per-formation lane policy profile (`lane priorities`, `lane quotas`, `fallbacks`).
  2. Apply formation-specific slot resolver behavior in assignment pass.
  3. Add policy for hybrid/special units (tracker/scout/flagellant/bannerman/hero subtypes) per formation.
  4. Add deterministic fallback for sparse rosters.
- Unit Tests Required:
  - Per-formation lane assignment tests for representative mixed rosters.
  - Regression tests ensuring no nondeterministic assignment drift under identical inputs.
- Acceptance Criteria:
  - Lane placement changes correctly when formation changes.
  - Special/hybrid units route to intended lanes per active formation.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-258 - Formation Progression and Skillbar Integration Pass
- Status: `DONE`
- Type: `UI`
- Priority: `P1`
- Depends on: `CRU-256`
- Goal: Integrate the expanded formation roster into skillbar unlock/progression and in-run readability.
- Context:
  - New formations need consistent unlock path, icons, descriptions, and modal presentation.
  - Existing skillbar and skill-book views currently assume two formations.
  - Files expected to change: `src/formation.rs`, `src/ui.rs`, `assets/data/upgrades.json`, formation icons/assets.
- Implementation:
  1. Add unlock path and skillbar slots for all new formations.
  2. Update skill-book and tooltip text with explicit tradeoffs and use-cases.
  3. Add icon and naming support for new formations.
  4. Ensure hotkey/selection UX scales beyond two formation choices.
- Unit Tests Required:
  - Skillbar/skill-book tests verify all formation entries can be added, selected, and displayed.
  - UI regression tests verify tooltip contract includes strengths/weaknesses for each formation.
- Acceptance Criteria:
  - Players can unlock/select/read all formation types consistently.
  - Formation UI remains clear and stable with expanded roster.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-259 - Formation Balance and Counterplay QA Matrix
- Status: `DONE`
- Type: `QA`
- Priority: `P0`
- Depends on: `CRU-256, CRU-257, CRU-258`
- Goal: Validate expanded formation roster for strategic differentiation and non-degenerate balance.
- Context:
  - Formation effects now include anti-entry, reflect, movement-state multipliers, and spacing changes.
  - Requires scenario-based validation across enemy compositions and difficulties.
  - Files expected to change: tests in `src/*`, QA docs/checklists.
- Implementation:
  1. Build matchup scenarios for each formation vs ranged-heavy, cavalry-heavy, mixed, and swarm enemy patterns.
  2. Validate counterplay exists for each formation (no universally dominant default).
  3. Tune effect magnitudes and caps from test outcomes.
  4. Record recommended usage windows in docs.
- Unit Tests Required:
  - Scenario tests asserting formation-specific expected outcomes under fixed seeds.
  - Regression tests preventing reflect/anti-entry/block interactions from exceeding bounds.
- Acceptance Criteria:
  - Each formation has a clear strategic identity with meaningful tradeoffs.
  - No formation trivially outperforms others across all scenarios.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-260 - Full Skill/Upgrade Audit Against New Pacing and Systems
- Status: `DONE`
- Type: `Docs`
- Priority: `P0`
- Depends on: `CRU-200, CRU-202, CRU-203, CRU-255`
- Goal: Perform a formal audit of all skills/upgrades against new level pacing, economy, formation roster, and counter-matrix goals.
- Context:
  - Existing revamp cards define direction, but system changes since then can cause drift and stale assumptions.
  - Audit should classify every skill/upgrade as keep/rework/remove.
  - Files expected to change: `docs/SYSTEMS_REFERENCE.md`, `docs/requirements.md`, `docs/TASKS.md`.
- Implementation:
  1. Inventory all active upgrades and skill effects with ownership systems and dependencies.
  2. Evaluate each for strategic impact, redundancy, contradiction, and balance risk under current run pacing.
  3. Produce disposition list (`keep`, `merge`, `rework`, `deprecate`) with rationale.
  4. Generate follow-up implementation cards for each rework/deprecation cluster.
- Unit Tests Required:
  - `none (audit/docs task)`
  - `none (audit/docs task)`
- Acceptance Criteria:
  - Every skill/upgrade has an explicit disposition and next action.
  - No high-impact legacy effect remains unreviewed.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/requirements.md`

### CRU-261 - Skill and Upgrade Schema Refactor Pass (Post-Audit Cutover)
- Status: `DONE`
- Type: `Core`
- Priority: `P0`
- Depends on: `CRU-260`
- Goal: Refactor skill/upgrade data schema and runtime to match audit decisions and remove stale mechanics.
- Context:
  - Requires schema/runtime changes where effects are deprecated, merged, or re-authored.
  - Must keep deterministic draft behavior and compatibility with major/minor cadence.
  - Files expected to change: `src/upgrades.rs`, `src/data.rs`, `assets/data/upgrades.json`, skill-related UI/data paths.
- Implementation:
  1. Apply schema updates for retained/reworked effect definitions.
  2. Remove deprecated effect handlers and dead UI metadata.
  3. Add migration validators for forbidden legacy fields/effects.
  4. Ensure draft and skill-book surfaces consume updated schema consistently.
- Unit Tests Required:
  - Schema validation tests for new/legacy effect forms.
  - Runtime effect tests for reworked high-impact skills/upgrades.
- Acceptance Criteria:
  - Runtime uses only post-audit skill/upgrade definitions.
  - Removed mechanics are no longer loadable or executable.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-262 - Skill/Upgrade Rebalance and Regression Certification
- Status: `TODO`
- Type: `QA`
- Priority: `P1`
- Depends on: `CRU-261, CRU-210, CRU-211`
- Goal: Certify that audited/refactored skills and upgrades are balanced, strategic, and regression-safe.
- Context:
  - This pass validates strategic value under limited level budget and expanded formation options.
  - Requires deterministic scenario coverage and manual QA heuristics.
  - Files expected to change: tests in `src/*`, QA docs/checklists.
- Implementation:
  1. Build targeted test scenarios for each major doctrine path and formation pairing.
  2. Validate pick value contrast (major vs minor) and absence of dead/on-rails picks.
  3. Validate economy and pacing interactions with gold/token/chest changes.
  4. Publish final tuning deltas and residual risks.
- Unit Tests Required:
  - Deterministic doctrine-path tests for representative build archetypes.
  - Regression tests for previously identified exploit or runaway interactions.
- Acceptance Criteria:
  - Skill/upgrade ecosystem is strategically differentiated and stable.
  - No known high-severity balance regressions remain open.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-263 - Integer-First Combat Baseline Refactor (Low-Number Scale)
- Status: `TODO`
- Type: `Core`
- Priority: `P0`
- Depends on: `CRU-255, CRU-260`
- Goal: Convert core combat values to low-range, integer-first baselines while preserving deterministic behavior.
- Context:
  - New direction uses low visible power ranges (for example single-digit damage bands) with hidden backend values.
  - Current combat stack mixes larger float-derived multipliers and percentage-heavy scaling.
  - Files expected to change: `src/combat.rs`, `src/model.rs`, `src/data.rs`, unit/enemy tuning data assets.
- Implementation:
  1. Define integer-first baseline ranges for core stats (`hp`, `damage`, `armor`, `block chance points`, `crit chance points`, `crit damage points`).
  2. Normalize combat calculations to additive point math where possible, with bounded conversions to runtime float operations only when required.
  3. Rebalance unit/enemy baseline profiles to low-number scale while preserving role identity.
  4. Add migration notes and validators for deprecated high-scale assumptions.
- Unit Tests Required:
  - Deterministic combat regression tests verifying identical outcomes under fixed seeds after scale conversion.
  - Baseline profile validation tests ensuring all core stats fall within approved integer-scale bounds.
- Acceptance Criteria:
  - Core combat runs on low-number integer-first semantics.
  - No system-critical path depends on legacy high-scale assumptions.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-264 - Additive-Only Upgrade Math Cutover (Remove Multiplicative Drift)
- Status: `TODO`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-263, CRU-261`
- Goal: Ensure upgrades/skills primarily apply additive point deltas and remove multiplicative stacking drift.
- Context:
  - Player direction is additive outcomes (`+1 damage`, `+2 crit chance`) instead of percentage-first scaling.
  - Existing legacy effects may still stack multiplicatively in some paths.
  - Files expected to change: `src/upgrades.rs`, `src/combat.rs`, upgrade data assets, skill metadata/UI text.
- Implementation:
  1. Replace percentage-based primary upgrade effects with additive point effects.
  2. Restrict multiplicative effects to explicitly whitelisted exceptional mechanics with hard caps.
  3. Update upgrade descriptions and skill-book metadata to additive phrasing.
  4. Add validators rejecting non-whitelisted multiplicative effect definitions.
- Unit Tests Required:
  - Upgrade application tests verifying additive accumulation and cap behavior.
  - Schema validator tests rejecting forbidden multiplicative effect forms.
- Acceptance Criteria:
  - Primary upgrade ecosystem is additive and predictable.
  - Multiplicative runaway interactions are removed or hard-bounded.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-265 - Hidden Numeric Bands + Threshold-Crossing UX Contract
- Status: `DONE`
- Type: `UI`
- Priority: `P0`
- Depends on: `CRU-215, CRU-263, CRU-264`
- Goal: Keep backend numeric values hidden while surfacing stat-band changes and threshold crossings clearly to players.
- Context:
  - Player-facing model should remain abstract/qualitative while backend keeps numeric precision.
  - Need explicit UI signaling when additive changes cross a band boundary.
  - Files expected to change: `src/ui.rs`, tooltip/status render helpers, docs.
- Implementation:
  1. Define per-stat-family band thresholds mapped from backend numeric points.
  2. Render trait-first plus band-first outputs (no raw combat numbers in primary surfaces).
  3. Add threshold-crossing feedback events (`Low -> Moderate`, etc.) on upgrade/item changes.
  4. Keep optional advanced/debug views gated and non-default.
- Unit Tests Required:
  - Band-mapping tests for each core stat family and boundary edge cases.
  - UI feedback tests confirming threshold-crossing messages trigger only on actual band transitions.
- Acceptance Criteria:
  - Players receive clear qualitative stat information and progression feedback.
  - Backend numeric values remain hidden in primary gameplay UI.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-266 - Shielded Trait and Block-Capable Unit Contract
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-247, CRU-248, CRU-263`
- Goal: Introduce explicit `Shielded` trait badge and make block chance exclusive to block-capable (`Shielded`) units.
- Context:
  - Player direction: only shield-bearing lines should have block chance and receive shield-wall block bonuses.
  - Current block behavior still relies on broader line checks.
  - Files expected to change: `src/model.rs`, `src/combat.rs`, `src/data.rs`, UI trait presentation paths.
- Implementation:
  1. Add explicit `Shielded` trait/tag in unit/hero profile data.
  2. Gate block stat/roll logic behind `Shielded` capability.
  3. Update tooltip/trait badges and formation interactions to reference block-capable units via trait.
  4. Add migration pass for current units that should/shouldn’t carry `Shielded`.
- Unit Tests Required:
  - Block gating tests verifying non-shielded units never roll block.
  - Trait-resolution tests verifying shielded line mapping and UI badge exposure.
- Acceptance Criteria:
  - Block mechanics are strictly trait-gated to shielded units.
  - Shield Wall block bonuses apply only to `Shielded` units.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-267 - Formation Rule Enforcement Semantics (Hard Anti-Entry + Reflect Rules)
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-255, CRU-256, CRU-266`
- Goal: Enforce exact anti-entry and reflect semantics for reworked formations.
- Context:
  - Player constraints:
    - anti-entry formations must enforce hard `0` enemies inside (`Shield Wall`, `Diamond`),
    - reflect uses post-armor incoming damage,
    - reflected damage mirrors crit-influenced incoming result but reflected hit itself cannot crit.
  - Files expected to change: `src/formation.rs`, `src/combat.rs`, `src/collision.rs`, docs.
- Implementation:
  1. Enforce hard anti-entry occupancy behavior and robust edge fallback for pathing congestion.
  2. Implement reflect calculation from post-mitigation hit result.
  3. Prevent reflected packets from rolling crits while preserving source-hit crit impact in reflected amount.
  4. Add explicit formation-rule tests and invariants.
- Unit Tests Required:
  - Anti-entry tests asserting inside enemy count remains `0` for protected formations.
  - Reflect tests covering regular hit, crit hit, and non-crit reflected packet behavior.
- Acceptance Criteria:
  - Anti-entry and reflect behaviors match the agreed semantics exactly.
  - No ambiguity remains in combat logs/tests for reflected damage behavior.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-268 - Formation Unlock Economy Rework (Square Default, Others via Upgrades)
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-255, CRU-258, CRU-264`
- Goal: Keep `Square` as the always-available default and unlock other formations through upgrades (major-first policy).
- Context:
  - Player direction: formations should be strategic progression choices, not fully available at run start.
  - Unlock flow should align with revised major/minor cadence and draft quality goals.
  - Files expected to change: `assets/data/upgrades.json`, `src/upgrades.rs`, `src/formation.rs`, `src/ui.rs`.
- Implementation:
  1. Keep `Square` unlocked by default at run start.
  2. Define formation unlock upgrades (major-lane primary; optional minor exceptions only by explicit design).
  3. Update draft gating/filters so locked formations cannot appear in skillbar selection.
  4. Ensure skill-book and tooltips show locked/unlocked formation states and requirements.
- Unit Tests Required:
  - Progression tests verifying `Square` default availability and gated unlocks for all other formations.
  - Draft/filter tests verifying locked formation upgrades follow lane and requirement policies.
- Acceptance Criteria:
  - `Square` is baseline in every run.
  - Non-default formations are unlocked only through configured upgrade flow.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-269 - Trait-Gated Upgrade Authoring Pass (Bracket/Tag Conditional Effects)
- Status: `DONE`
- Type: `Balance`
- Priority: `P1`
- Depends on: `CRU-247, CRU-264, CRU-265, CRU-266`
- Goal: Author upgrades that target stat brackets and traits (for example low-armor -> shielded + block) under additive balance rules.
- Context:
  - Player direction includes conditional upgrades keyed to abstract brackets/traits rather than raw stat percentages.
  - Needs consistent interaction with hidden numeric thresholds and trait tags.
  - Files expected to change: `assets/data/upgrades.json`, `src/upgrades.rs`, `src/ui.rs`.
- Implementation:
  1. Add requirement types for stat-band and trait predicates (`has_trait`, `band_at_most`, `band_at_least`).
  2. Author first pass of bracket/trait-gated upgrades across infantry/ranged/support/hero lines.
  3. Ensure UI clearly explains eligibility and current active/inactive reasons.
  4. Balance additive values to avoid runaway band skipping.
- Unit Tests Required:
  - Requirement evaluator tests for trait+band condition combinations.
  - Runtime activation tests for representative upgrades (including shielded-conversion style effects).
- Acceptance Criteria:
  - Upgrade pool contains meaningful trait/bracket-targeted choices.
  - Conditional upgrade behavior is deterministic and clearly communicated in UI.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-241 - New-Faction Onboarding Harness (Config-Only Expansion Gate)
- Status: `DONE`
- Type: `QA`
- Priority: `P1`
- Depends on: `CRU-239, CRU-240, CRU-244`
- Goal: Add automated validation proving that adding a faction is configuration-first and does not require system rewiring.
- Context:
  - Primary business outcome for this refactor is scalable faction onboarding.
  - Need a CI gate that catches hidden code assumptions tied to old faction-duplicated enums.
  - Files expected to change: tests in `src/*`, validation fixtures under `assets/data/test`.
- Implementation:
  1. Add synthetic third-faction fixture data using generic unit/hero IDs and override files.
  2. Add loader/validation tests that pass with fixture faction and fail with deliberately broken references.
  3. Add smoke runtime test to ensure core systems can resolve roster/wave/recruit/tooltips and item drops/icons for fixture faction.
  4. Add checklist docs for future faction onboarding.
- Unit Tests Required:
  - Third-faction fixture load test (valid configuration).
  - Third-faction failure tests (missing overrides, bad IDs, broken promotion/drop references).
- Acceptance Criteria:
  - CI proves a new faction can be introduced via data only.
  - No hidden hardcoded two-faction assumptions remain in tested runtime paths.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-242 - Legacy Cleanup Cutover (Remove Faction-Duplicated Unit/Hero/Item Definitions)
- Status: `DONE`
- Type: `Core`
- Priority: `P0`
- Depends on: `CRU-241`
- Goal: Remove legacy faction-duplicated unit/hero/item code paths and finalize migration to generic IDs only.
- Context:
  - User direction is no backwards compatibility for cohesion-like legacy paths; the same applies here after migration is validated.
  - Keeping dual-path code increases maintenance and regression risk.
  - Files expected to change: `src/model.rs`, `src/data.rs`, `src/squad.rs`, `src/ui.rs`, `src/inventory.rs`, `src/drops.rs`, old data files.
- Implementation:
  1. Delete deprecated faction-specific unit/hero/item identity definitions and dead adapters.
  2. Remove legacy config loaders and stale JSON fields.
  3. Update all docs/tests to reference only generic ID architecture.
  4. Run full regression loop and fix remaining migration issues.
- Unit Tests Required:
  - Compile-time/routing tests ensure no legacy identity path is referenced.
  - Regression suite verifies gameplay parity after legacy-path removal.
- Acceptance Criteria:
  - Repository has one identity model: generic IDs with faction overrides.
  - No runtime path depends on faction-duplicated unit definitions.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-233 - Faction-Specific Hero Pools and Name Rosters
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-240, CRU-242`
- Goal: Add faction-specific hero pools with random recruit selection per chosen subtype, including `10` names per subtype per faction.
- Context:
  - This card now becomes a content-authoring pass on top of the generic hero runtime.
  - Hero recruit action selects hero type/subtype, then rolls one hero from that faction's subtype pool.
  - Duplicate heroes are allowed by design.
  - Required subtype pools per faction: melee (`sword_shield`, `spear`, `two_handed_sword`), ranged (`bow`, `javelin`, `beast_master`), support (`super_priest`, `super_fanatic`), cavalry (`super_knight`).
  - Files expected to change: `assets/data/heroes.json` (or equivalent), `src/data.rs`, `src/squad.rs`, `src/ui.rs`.
- Implementation:
  1. Add hero data schema for faction, subtype, display name, role tags, and optional trait hooks.
  2. Author hero name rosters with exactly `10` entries per subtype per faction.
  3. Implement weighted-random recruit resolution from selected faction + subtype pool with duplicates allowed.
  4. Expose selected hero identity in roster and upgrade UI after recruitment.
- Unit Tests Required:
  - Data validation test enforces exactly `10` names per subtype per faction.
  - Recruit selection test verifies random roll draws from selected faction/subtype and allows duplicates.
- Acceptance Criteria:
  - Both factions have complete hero-name pools for every defined hero subtype.
  - Hero recruit flow returns valid faction-matching heroes only.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-234 - Integrated Balance and QA Pass for Army/Tier/Hero Expansion
- Status: `TODO`
- Type: `QA`
- Priority: `P0`
- Depends on: `CRU-223, CRU-224, CRU-225, CRU-232, CRU-233, CRU-242, CRU-244`
- Goal: Validate end-to-end stability and balance for difficulty modes, wave-army cadence, tier unlock flow, hero recruitment, and deterministic progression parity.
- Context:
  - Expansion touches wave scheduler, AI, progression unlocks, roster graph, resources, item-drop/icon resolvers, and UI.
  - Requires deterministic replay coverage across all three difficulty profiles and both factions.
  - Files expected to change: tests in `src/*`, QA docs/checklists.
- Implementation:
  1. Add end-to-end replay tests for waves `1..60` and `1..98` per difficulty profile.
  2. Validate tier unlock milestones are boss-defeat gated and never wave-index-only.
  3. Validate hero unlock/token flows and faction-specific hero pool integrity in live run paths.
  4. Validate faction-aware item drops/icons and run balance sweeps to ensure no single difficulty strategy or unit branch trivially dominates.
- Unit Tests Required:
  - End-to-end progression test verifying major/minor parity and boss-gated tier unlock progression.
  - End-to-end hero/item test verifying wave-60 unlock gate, token consumption, faction pool correctness, and faction item-drop/icon resolution.
- Acceptance Criteria:
  - Army/difficulty/tier/hero systems are stable under automated and manual QA matrices.
  - Expansion is ready for release packaging without regression against revamp foundations.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-270 - Floating Combat Text Signal-Only Pass (`Block!` / `Critical Hit!`)
- Status: `DONE`
- Type: `UI`
- Priority: `P0`
- Depends on: `CRU-263, CRU-264`
- Goal: Limit floating combat text to high-signal events only (`Block!`, `Critical Hit!`) and remove routine damage-number noise.
- Context:
  - Integer-first combat and hidden numeric presentation reduce the need for per-hit damage numbers in primary combat UX.
  - Player direction is explicit: only block and critical should produce combat text.
  - Files expected to change: `src/model.rs`, `src/combat.rs`, `src/ui.rs`.
- Implementation:
  1. Replace numeric damage-text payload semantics with explicit event kinds (`blocked`, `critical_hit`).
  2. Emit `blocked` text events on blocked hits and `critical_hit` events on critical landed hits.
  3. Remove regular numeric/execute floating text rendering from the default combat loop.
  4. Keep text styling distinct and readable at swarm scale.
- Unit Tests Required:
  - Combat event emission tests confirm `blocked` and `critical_hit` are emitted only under correct conditions.
  - UI spawn-data tests confirm text labels and styling are correct for both event kinds.
- Acceptance Criteria:
  - Normal non-critical hits show no floating damage numbers.
  - `Block!` and `Critical Hit!` are the only default floating combat-text signals.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-271 - Segmented Health Bar Threshold-Snap Rendering
- Status: `DONE`
- Type: `UI`
- Priority: `P0`
- Depends on: `CRU-265`
- Goal: Render health bars as snapped segments tied to stat-band thresholds instead of continuous fill values.
- Context:
  - Player-facing readability direction favors abstracted bracket feedback over precise raw numeric depletion.
  - Requested behavior: health fill snaps to the next segment only when threshold boundaries are crossed.
  - Files expected to change: `src/ui.rs`, health-bar helper tests, relevant HUD docs.
- Implementation:
  1. Add segmented-fill helper logic (canonical segment count and threshold snap policy).
  2. Replace continuous health fill rendering with snapped segment widths.
  3. Ensure threshold transitions are stable on both damage and healing updates.
  4. Keep team color semantics and existing bar entity structure intact.
- Unit Tests Required:
  - Segment snap tests for threshold edges (`full`, `boundary`, `below-boundary`, `zero`).
  - Fill width tests verify clamping and monotonic segmented behavior under invalid/overflow input.
- Acceptance Criteria:
  - Health bars change only on segment-threshold crossing and no longer animate continuously per point loss.
  - Segment behavior is deterministic and test-covered at boundary edges.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-272 - Combat Readability Regression and UX Certification (Signal Text + Segments)
- Status: `DONE`
- Type: `QA`
- Priority: `P1`
- Depends on: `CRU-270, CRU-271`
- Goal: Certify readability and gameplay clarity after removing numeric damage text and introducing segmented health bars.
- Context:
  - UI feedback changes can affect perceived combat responsiveness and balance readability.
  - This pass validates both deterministic correctness and practical combat readability.
  - Files expected to change: test suites in `src/*`, QA notes/checklists.
- Implementation:
  1. Add deterministic scenarios covering mixed crit/block frequency under high unit counts.
  2. Validate that floating text volume remains bounded and legible in late-wave swarms.
  3. Validate health-segment transitions across diverse max-health profiles and rapid heal/damage cycles.
  4. Capture residual readability risks and tuning recommendations.
- Unit Tests Required:
  - Stress test for floating-text spawn caps under crit/block-heavy combat.
  - Segmented health-transition regression tests for repeated boundary oscillation.
- Acceptance Criteria:
  - No regressions in combat readability or UI stability are observed in automated and manual QA coverage.
  - UX behavior aligns with abstract stat-band presentation goals.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-273 - Upgrade Catalog Consolidation (Doctrine Families + Economy Ladder)
- Status: `DONE`
- Type: `Core`
- Priority: `P0`
- Depends on: `CRU-260`
- Goal: Consolidate duplicate upgrade IDs into schema-driven families without losing doctrine variety.
- Context:
  - Audit identified parallel duplicate IDs that encode variants in ID names instead of data fields (`fast_learner_up_10`, `fast_learner_up_15`, doctrine variant suffixes; now consolidated under `quartermaster_up`).
  - Consolidation should reduce maintenance overhead and simplify balancing under major/minor pacing.
  - Files expected to change: `assets/data/upgrades.json`, `src/data.rs`, `src/upgrades.rs`, UI metadata mapping.
- Implementation:
  1. Define canonical schema fields for doctrine/economy variant tuning (weights, thresholds, trait predicates, caps) under one family ID.
  2. Collapse duplicate legacy IDs into consolidated entries while preserving deterministic draft behavior.
  3. Update runtime resolver and skill-book rendering to consume consolidated variant metadata.
  4. Add migration notes mapping old IDs to consolidated family entries.
- Unit Tests Required:
  - Data-validation tests ensuring consolidated families load and old duplicate IDs are not required.
  - Runtime selection tests ensuring deterministic variant resolution under fixed seeds.
- Acceptance Criteria:
  - Duplicate upgrade IDs are removed from active authoring and replaced by consolidated family definitions.
  - Doctrine/economy variants remain selectable and deterministic through data-driven fields.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

### CRU-274 - Legacy Upgrade ID Deprecation Guards and Cutover
- Status: `DONE`
- Type: `Core`
- Priority: `P0`
- Depends on: `CRU-261, CRU-273`
- Goal: Enforce hard deprecation of audited legacy upgrade IDs and prevent reintroduction.
- Context:
  - Post-consolidation, legacy duplicate IDs must become invalid inputs to avoid drift and accidental regressions.
  - Deprecated IDs from audit: `fast_learner_up`, `fast_learner_up_10`, `fast_learner_up_15`, `mob_fury_shielded_host`, `mob_justice_frontline_bias`, `mob_mercy_support_ceiling`.
  - Files expected to change: `src/data.rs`, `src/upgrades.rs`, upgrade validators/tests, docs.
- Implementation:
  1. Add explicit validator blacklist for deprecated upgrade IDs and fail-fast load errors.
  2. Remove any runtime handling paths that reference deprecated IDs.
  3. Add migration notes and diagnostic error text pointing to consolidated replacements.
  4. Add regression tests preventing deprecated IDs from loading in fixtures/assets.
- Unit Tests Required:
  - Loader tests asserting deprecated IDs are rejected with clear errors.
  - Regression tests ensuring no runtime resolver path accepts deprecated IDs.
- Acceptance Criteria:
  - Deprecated IDs cannot be authored or loaded.
  - Runtime and docs reference only consolidated post-audit upgrade identities.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md`

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
