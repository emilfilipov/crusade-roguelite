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
- Status: `TODO`
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
- Status: `TODO`
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
- Status: `IN PROGRESS`
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
- Status: `TODO`
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
- Status: `TODO`
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
- Status: `TODO`
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
- Status: `TODO`
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
- Status: `TODO`
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
- Status: `TODO`
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
- Status: `TODO`
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
- Status: `TODO`
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
- Execution order (high level): `CRU-218 -> CRU-219 -> CRU-220 -> CRU-221 -> (CRU-222 + CRU-223 + CRU-224) -> CRU-225 -> (CRU-226 -> CRU-227 -> CRU-228 -> CRU-229 -> CRU-230) -> CRU-231 -> (CRU-232 + CRU-233) -> CRU-234`.
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
- Status: `TODO`
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
- Status: `TODO`
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
- Status: `TODO`
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

### CRU-233 - Faction-Specific Hero Pools and Name Rosters
- Status: `TODO`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-231`
- Goal: Add faction-specific hero pools with random recruit selection per chosen subtype, including `10` names per subtype per faction.
- Context:
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
- Depends on: `CRU-223, CRU-224, CRU-225, CRU-232, CRU-233`
- Goal: Validate end-to-end stability and balance for difficulty modes, wave-army cadence, tier unlock flow, hero recruitment, and deterministic progression parity.
- Context:
  - Expansion touches wave scheduler, AI, progression unlocks, roster graph, resources, and UI.
  - Requires deterministic replay coverage across all three difficulty profiles and both factions.
  - Files expected to change: tests in `src/*`, QA docs/checklists.
- Implementation:
  1. Add end-to-end replay tests for waves `1..60` and `1..98` per difficulty profile.
  2. Validate tier unlock milestones are boss-defeat gated and never wave-index-only.
  3. Validate hero unlock/token flows and faction-specific hero pool integrity in live run paths.
  4. Run balance sweeps to ensure no single difficulty strategy or unit branch trivially dominates.
- Unit Tests Required:
  - End-to-end progression test verifying major/minor parity and boss-gated tier unlock progression.
  - End-to-end hero test verifying wave-60 unlock gate, token consumption, and faction pool correctness.
- Acceptance Criteria:
  - Army/difficulty/tier/hero systems are stable under automated and manual QA matrices.
  - Expansion is ready for release packaging without regression against revamp foundations.
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
