# SYSTEMS_REFERENCE.md

## Purpose
Single-file technical reference for current MVP runtime behavior.
Use this for entity/component/system lookup without scanning all source files.

## Latest Update (2026-03-30)
- Completed CRU-212/234/263/264 finalization tranche:
  - release sign-off artifacts added: `docs/REVAMP_RELEASE_CHECKLIST.md` and `docs/EXPANSION_QA_MATRIX.md`,
  - combat damage now resolves through integer-point projection before world-space application (low-range point semantics),
  - critical damage now applies as additive point gain over base point damage (instead of pure multiplier chaining),
  - friendly offensive/attack-speed stack composition now uses additive multiplier aggregation (reduced multiplicative drift),
  - semantic trait-effect validation now restricts multiplicative effect kinds to whitelisted doctrine upgrades.
- Completed CRU-209/210/211/262 stabilization tranche:
  - level-up draft cards now frame choices by reward lane (`Minor` vs `Major`) and surface explicit `Upside`, `Downside`, `Doctrine`, and `Notes` blocks,
  - lane visuals now use dedicated lane styling instead of legacy rarity-legend framing,
  - deterministic draft replay coverage was expanded (same seed + same inputs => same minor/major option sets),
  - doctrine-path regression coverage now explicitly includes pike-hedgehog tradeoff behavior (damage gain, move-speed penalty, morale-loss mitigation),
  - release QA matrix and pass/fail thresholds were formalized in `docs/REVAMP_QA_MATRIX.md`.
- Completed CRU-208/216/217 implementation tranche:
  - equipment scrap values are now authored per item template (`scrap_gold_value`) and no longer derived from runtime rarity math,
  - gold economy scaling/costs were rebalanced (drop scaling clamp + updated swap/promotion/hero costs) to preserve opportunity cost across full runs,
  - upgrade schema now supports semantic effect primitives (`effect_band_shift_*`, `effect_band_floor_*`, `effect_trait_modifier_*`) with strict validation,
  - level-up cards now show `current -> after` preview lines for supported semantic band-shift effects,
  - primary item/unit surfaces remain qualitative-first, and exact values are opt-in via `Advanced Details` toggle (`F3`).
- Completed CRU-205/206/207 revamp tranche:
  - Minor upgrades now support authored stack controls (`stack_cap`) plus diminishing returns (`diminishing_factor`) and are filtered from future drafts once capped.
  - Conditional upgrade requirements were tightened around roster commitment bands/traits (including new `anti_cavalry_share`) and conditional effects now dedupe by upgrade kind.
  - Item generation is now deterministic per template (fixed stat packages, fixed rarity labels, explicit downside + doctrine tags + `Minor/Major` nature) with no runtime rarity-roll stat generation.
- Added faction-agnostic identity scaffold in runtime model:
  - `UnitRef { faction, unit_id, rescuable }` is now derivable from any `UnitKind`,
  - shared `unit_id` strings now map Christian/Muslim/rescuable variants into one generic ID space (for example both faction peasant infantry variants resolve to `peasant_infantry`).
- Completed naming/presentation cutover for roster-facing labels:
  - `unit_kind_label` now returns generic unit names (no hardcoded `Christian`/`Muslim` prefixes),
  - rescuable variants now display explicit generic rescuable labels (`Rescuable Peasant Infantry/Archer/Priest`),
  - archive unit/enemy entries now use generic-first naming with profile variants in descriptions.
- Completed item/drop catalog + icon resolver cutover:
  - added data-driven item catalog at `assets/data/items.json` loaded on boot into `ItemTemplateCatalog`,
  - catalog loader now allows duplicate `item_id` values when scoped to different factions (for override-style entries),
  - chest rolls now use catalog templates via faction-aware weighted selection,
  - UI icon fallback now resolves through `icon_key` + faction (`item_symbol_faction`) instead of hardcoded symbol branches.
- Migrated units/enemies runtime schema to strict base+override contracts:
  - `assets/data/units.json` now resolves from `base { commander, recruits }` + `overrides.{faction}`,
  - `assets/data/enemies.json` now resolves from `base { profiles }` + `overrides.{faction}`,
  - override keys are validated (`faction`, `unit_id`) and unknown keys fail load with explicit parse errors,
  - legacy faction-duplicated top-level schema keys are rejected after cutover.
- Added generic archetype resolver pass in core data/runtime paths:
  - `UnitsConfigFile` now resolves recruit stats via `faction + archetype`,
  - `EnemiesConfigFile` now resolves enemy profiles via `faction + archetype`,
  - opposing tier-0 enemy pools and UI roster tier-0 source pools now derive from `RecruitUnitKind::all_for_faction(...)` instead of hardcoded per-faction arrays.
- Reduced faction-duplicated branch logic in runtime systems:
  - inventory combat-role resolution now keys off generic `unit_id` instead of explicit Christian/Muslim variant lists,
  - enemy sprite family mapping now keys off generic `unit_id` + faction presentation assets,
  - rescuable spawn presentation now resolves from `faction + archetype` helpers.
- Continued runtime identity cutover in squad/archive/rescue paths:
  - `friendly_stats_for_kind` now resolves from `tier + faction + unit_id` instead of duplicated Christian/Muslim tier branches,
  - tier3/4/5 source progression lookups now route through shared `unit_id` source maps + `UnitKind::from_faction_and_unit_id(...)`,
  - archive core unit/enemy profile entries now read through `faction + archetype` resolvers rather than direct faction-specific config fields.
- Migrated rescue pool schema to generic archetype IDs:
  - `assets/data/rescue.json` `recruit_pool` now uses `peasant_infantry|peasant_archer|peasant_priest` only,
  - rescue spawn selection resolves player-faction variants at runtime from generic pool entries,
  - pity counters now track archetype drought (infantry/archer/priest) instead of faction-specific recruit entries.
- Moved promotion graph resolution to generic IDs:
  - `promotion_targets_for_kind` now routes through a shared `unit_id -> target unit_id[]` graph and resolves faction variants via `UnitKind::from_faction_and_unit_id(...)`,
  - rescuable variants are explicitly non-promotable,
  - graph integrity tests now assert faction-scoped target resolution and one-tier progression for all mapped source nodes.
- Removed hardcoded wave fallback enemy identity:
  - enemy kind fallback selection now resolves the opposing faction's `peasant_infantry` via `UnitKind::from_faction_and_unit_id(...)`,
  - wave selection tests now cover faction-aware fallback behavior.
- Completed data-driven hero recruit resolver:
  - added `assets/data/heroes.json` with generic hero subtype definitions and faction override entry pools,
  - each hero subtype now resolves recruit candidates by `faction + subtype_id` (no hardcoded subtype->unit runtime mapping path),
  - recruit gold cost now resolves from hero subtype config, and recruit flow still keeps wave-60 unlock + `Hear the Call` token gating,
  - hero recruit tooltips now display subtype description, pool size, preview names, and configured abilities/stat notes.
- Added wave-tier composition ramp + major-army preview logic:
  - regular waves now blend `previous_tier -> unlocked_tier` by wave index (`11..20`, `21..30`, etc.) and reach `100%` unlocked-tier by the next major wave,
  - major-army lane now includes a difficulty-scaled next-tier preview share (`Recruit 20%`, `Experienced 35%`, `Alone 50%`) when a higher tier exists,
  - major-army preview count is now targeted per spawn batch from configured preview percentage (rounded, min `1` when preview is active),
  - wave pool builders now resolve enemy unit kinds from `assets/data/enemy_tier_pools.json` tier/role `unit_id` sets + faction context.
- Expanded enemy profile resolution for higher-tier enemy kinds:
  - `enemy_profile_for_kind(...)` now resolves non-tier0 lines by shared archetype traits (`infantry`/`archer`/`priest`) so wave-tier mixes can spawn upgraded unit kinds while preserving faction profile modifiers.
- Added `assets/data/roster_tuning.json` and wired it into `GameData`:
  - tier-2 unit stats are now data-driven per tier-2 unit kind,
  - tracker/scout autonomous behavior timings and multipliers are data-driven,
  - fanatic life-leech ratio is data-driven.
- Completed tier-2 branch runtime:
  - tracker timed hound-strike behavior and scout out-of-formation raid behavior are live,
  - fanatic branch now has hard `ArmorLockedZero` behavior (no armor from gear/upgrade layers in damage resolution),
  - fanatic life-leech on melee hit uses applied damage and branch-configured leech ratio.
- Unit Upgrade graph now exposes tier-2 promotion options in the tier-2 column for owned tier-1 sources.
- Completed tier-3 branch runtime for both factions:
  - tier-2 branches now continue into tier-3 (`experienced shield infantry`, `shielded spearman`, `knight`, `bannerman`, `elite bowman`, `armored crossbowman`, `pathfinder`, `mounted scout`, `cardinal`, `flagellant`),
  - tracker/scout branch actives carry forward (`Pathfinder` keeps hound strikes, `Mounted Scout` keeps raid behavior),
  - fanatic branch traits carry forward (`Flagellant` keeps armor-lock-at-zero + life-leech behavior).
- Completed tier-4 branch runtime for both factions:
  - tier-3 branches now continue into tier-4 (`elite shield infantry`, `halberdier`, `heavy knight`, `elite bannerman`, `longbowman`, `elite crossbowman`, `houndmaster`, `shock cavalry`, `elite cardinal`, `elite flagellant`),
  - tracker/scout branch actives carry forward (`Houndmaster` keeps hound strikes, `Shock Cavalry` keeps raid behavior),
  - fanatic branch traits carry forward (`Elite Flagellant` keeps armor-lock-at-zero + life-leech behavior).
- Completed tier-5 branch runtime for both factions:
  - tier-4 branches now continue into tier-5 (`citadel guard`, `armored halberdier`, `elite heavy knight`, `god's chosen`, `elite longbowman`, `siege crossbowman`, `elite houndmaster`, `elite shock cavalry`, `divine speaker`, `divine judge`),
  - tracker/scout branch actives carry forward (`Elite Houndmaster` keeps hound strikes, `Elite Shock Cavalry` keeps raid behavior),
  - fanatic branch traits carry forward (`Divine Judge` keeps armor-lock-at-zero + life-leech behavior).
- Added hero-tier unlock + `Hear the Call` token economy and recruit actions:
  - hero-tier unlock state now tracks major-army progression and flips at wave-60 major defeat,
  - `Hear the Call` now drops as a dedicated single-item chest reward (not gold-style auto pickup),
  - token chest drops now roll on enemy deaths across all waves using source-lane metadata (`small`/`minor`/`major`) with very-low lane-scaled RNG (`major > minor > small`) plus stash damping,
  - major-wave and lane-clear reward handlers no longer inject deterministic token grants; token flow is fully lane/death RNG driven,
  - army-lane clear rewards continue to roll equipment chests independently of token drops,
  - periodic random world equipment chests remain active on the existing cadence (every 3 waves),
  - equipment chests now roll deterministic item templates (fixed stats/tradeoffs); RNG selects which template drops, not stat magnitudes,
  - token chest pickups add `+1` `Hear the Call` to progression and HUD/unit-upgrade surfaces show token count,
  - token chest channel bars and minimap markers now render with distinct colors from equipment chests for pickup readability,
  - hero subtype buttons are now actionable in `Unit Upgrade` and spend `1` token per recruit.
- Synced wave runtime docs to current code:
  - 30s wave windows, `MAX_WAVES=100`,
  - spawn-rate clamp now uses `MAX_ENEMIES_PER_WAVE=200`,
  - stat growth slope is `+0.102` per wave step,
  - queued batch emission uses `batch_size = clamp(7 + wave/4, 7, 22)` and
    `batch_interval = clamp(0.7 - wave*0.01, 0.24, 0.7)`.
- Synced morale runtime docs to the single-active-morale implementation:
  - high-morale bracket grants gradual damage/armor/regen bonuses,
  - low-morale bracket applies armor penalty and escape-speed bonus,
  - collapse triggers at average morale `<= 0` with delayed reset and retinue loss.
- Fixed morale runtime integration gaps:
  - collapse trigger now uses the true average friendly morale ratio (so zero-morale collapse can fire),
  - authority/hospitalier aura effects now apply directly to live morale drain/regen paths,
  - commander banner morale stats now affect morale flow while active and are removed when the banner drops.
- Synced gold economy + deterministic wave-level docs to current values:
  - no XP thresholds; level rewards are queued from `WaveCompletedEvent`,
  - each completed wave grants `+1` level reward, and wave `98` grants `+2` (reaches level 100 at wave 98 completion),
  - reward queue now stores explicit reward kinds and drains in deterministic FIFO order (`Minor` per level, `Major` when resulting level `% 5 == 0`),
  - drop gold scaling now uses `base * clamp((1 + 0.03*(wave-1)) * (1 + 0.012*(level-1)), 1.0, 4.5)`,
  - roster level budget cap uses `MAX_COMMANDER_LEVEL=100` (`100 - locked_levels`, saturating).
- Reworked in-run `Unit Upgrade (U)` modal into a tier-column graph:
  - columns `Tier 0..Tier 5` with thin borders and row-wise straight connectors from tier-0 to tier-1,
  - tier-0 nodes are active source units,
  - tier-1 nodes are now active promotion targets,
  - tier-2 nodes are now active branch targets for owned tier-1 source kinds,
  - tier-3 nodes are now active continuation targets for owned tier-2 branch kinds,
  - tier-4 nodes are now active continuation targets for owned tier-3 branch kinds,
  - tier-5 nodes are now active continuation targets for owned tier-4 branch kinds,
  - `Hero` nodes now expose subtype recruit buttons with unlock/token gating and disabled-state reasons.
- Updated unit-upgrade node labeling:
  - unit boxes now render unit name only (no tier/count text inside the node).
- Added per-tier0 swap controls as row actions:
  - each tier-0 source row has a target selector (`dropdown-like` cycle control),
  - each row has a `Swap 1` action wired to `ConvertTierZeroUnitsEvent`,
  - row status includes source/target counts, target-option count, affordability, and gold cost.
- Added `Unit Upgrade` hover tooltip overlay:
  - hovering unit nodes now shows `Name`, `Type`, `Description`, `Stats`, and `Abilities`,
  - stats now render as qualitative 5-band descriptors (`|....` to `|||||`) instead of raw numeric stat dumps in the default tooltip surface,
  - tooltip trait badges now explicitly show `Shielded` when applicable,
  - scaffold nodes provide explicit placeholder metadata and tier-rule guidance.
- Added stat-band threshold feedback to the in-run toast channel:
  - monitors qualitative bands for `Damage`, `Armor`, `Move Speed`, and `Luck`,
  - emits explicit transition messages (for example `Damage: Low -> Moderate`) when bands change due upgrade/equipment state changes.
- Added faction-agnostic counter-matrix runtime hooks:
  - `UnitKind` now exposes tag-like role metadata (`frontline`, `anti_cavalry`, `cavalry`, `anti_armor`, `skirmisher`, `support`, `hero_doctrine`) and an `armor_class` resolver from shared `unit_id`,
  - combat now applies bounded matchup multipliers through `role_counter_damage_multiplier(attacker_kind, defender_kind)` for both melee and projectile hits,
  - projectile packets now carry `source_kind` so ranged counters resolve against the actual hit target kind.
- Added hero subtype specialization runtime pass:
  - recruited heroes now receive a `HeroSubtypeUnit` marker storing selected subtype,
  - each subtype has explicit combat tradeoff profile (`outgoing`, `incoming`, `attack_speed`) plus matchup-specific modifiers,
  - hero subtype modifiers now apply in melee and projectile combat paths, and in attack timer scaling.
- Expanded trait-first tooltip clarity:
  - unit tooltips now include `Strengths`, `Weaknesses`, and `Key Matchups` sections based on role tags/counter matrix,
  - hero recruit subtype tooltips now include the same sections from subtype doctrine mappings.
- Enabled Tier-1 promotion runtime in `U`:
  - each tier-0 source row now has one active tier-1 promotion target (`+1` promotion action),
  - promotion buttons are gated by boss-tier unlock state, source count, treasury, and level-budget affordability,
  - tier-2 branch buttons are now rendered in the tier-2 column for owned tier-1 kinds.
- Tier-1 roster branch contract (implemented):
  - `Christian Peasant Infantry -> Christian Men-at-Arms`,
  - `Christian Peasant Archer -> Christian Bowman`,
  - `Christian Peasant Priest -> Christian Devoted`,
  - `Muslim Peasant Infantry -> Muslim Men-at-Arms`,
  - `Muslim Peasant Archer -> Muslim Bowman`,
  - `Muslim Peasant Priest -> Muslim Devoted`.
- Added unit tests for new unit-upgrade logic:
  - swap-target fallback and cycling behavior,
  - unit-tooltip contract coverage for required sections.
- Added data-driven faction gameplay edge config in `assets/data/factions.json`:
  - per-faction friendly modifiers (HP, damage, attack speed, move speed, armor bonus, morale baseline),
  - per-faction morale flow modifiers (gain/loss scaling),
  - per-faction gold gain multiplier and rescue-time multiplier,
  - per-faction authority-aura enemy morale-drain multiplier tuning,
  - per-faction enemy-side modifiers applied when that faction is spawned as enemies (HP/damage/attack speed/move speed/morale).
- Commander aura radius now resolves by selected faction commander profile + faction aura bonus (instead of maxing Christian/Muslim commander aura radius).
- Friendly unit spawn/promotion stat setup now consumes faction modifiers, so faction identity applies consistently to commander and retinue.
- Enemy spawn stat setup now consumes the spawned enemy faction's modifiers, enabling asymmetric Christian vs Muslim enemy behavior tuning.
- Gold pickup gain and rescue-channel duration now include selected-faction multipliers.
- Added dual-faction runtime scaffold:
  - playable factions: `Christian` and `Muslim`,
  - selected faction controls commander + rescue recruit pool,
  - enemy waves draw from the opposite faction pool.
- Added Muslim roster/commander assets and wiring:
  - `Saladin` commander profile + sprite,
  - `muslim_peasant_infantry`, `muslim_peasant_archer`, `muslim_peasant_priest`,
  - Muslim rescuable variants and faction-aware pity-weighted rescue spawning.
- Replaced single `bandit_raider` enemy schema with faction-mirrored enemy profiles (Christian + Muslim infantry/archer/priest entries).
- Formation footprint occupancy cap now uses a strict retinue ratio: `floor(retinue_count / 4)` enemies allowed inside.
- Floating combat text is signal-only:
  - `BLOCK!` when an enemy successfully blocks,
  - `CRITICAL HIT!` on landed critical hits,
  - regular damage numbers and execute text are not shown in default combat feedback.
- Stats modal table now reports aggregated stat bonuses by stat name (for example `Health`, `Damage`, `Morale Regen/s`, `Morale Loss Resist`) instead of effect-source row names.
- Collision pause artifact fix: collision correction now applies `0` movement when simulation `delta_seconds == 0` (modal/paused virtual-time frames), preventing enemies from spreading while menus are open.
- Removed legacy enemy data dependency in runtime enemy config:
  - `enemies.json` schema now relies on morale plus core combat/movement stats,
  - enemy spawn no longer consumes deprecated legacy values from data.
- Rescue spawn selection now uses pity-weighted randomness:
  - each rescue type gains spawn weight the longer it has not spawned,
  - the spawned type resets its drought counter to `0`,
  - other types in the active rescue pool increment drought each spawn.
- Added `RunModalState` state machine for in-run utility screens (`Inventory`, `Stats`, `Skill Book`, `Archive`, `Unit Upgrade`).
- Added shared modal request event path (`RunModalRequestEvent`) so keyboard and UI button actions use the same reducer logic.
- Added modal hotkeys in-run: `I`, `O`, `K`, `B`, `U`; `Escape` closes modal first, otherwise opens pause menu.
- Added responsive UI scaling based on current primary-window resolution (reference `1280x720`, clamped range `0.7..3.0`) to keep HUD/modals usable across windowed/fullscreen/borderless modes.
- Added pause-state `Escape` behavior: pressing `Escape` while paused resumes the run.
- Added modal overlay scaffold renderer that pauses in-run simulation while open.
- Added direct close-button modal state clear path to avoid stale open overlays from UI interaction edge cases.
- Stabilized Unit Upgrade modal close/interaction behavior by removing per-frame refresh churn from promotion feedback updates.
- Added top-right in-run utility bar with five icon buttons mapped to the same modal requests as hotkeys.
- Added in-run commander aura footprint gizmo for clearer aura coverage.
- Formation slot resolver now uses a lane-policy model (`outer`, `middle`, `inner`) with deterministic quota assignment:
  - lane quotas are computed per active roster footprint and reserve outer shell capacity for frontline pressure first,
  - quotas are formation-profiled (`Shield Wall` outer-heavy shell, `Loose` middle-heavy spread, `Circle` balanced shell, `Skean`/`Diamond` momentum-frontloaded),
  - lane preferences are resolved from unit traits/tags (`frontline`, `anti_cavalry`, `cavalry`, `skirmisher`, `support`, `shielded`) instead of faction-duplicated unit-match tables,
  - support lines prefer `inner -> middle -> outer` and are capped on outer-shell crowding when other lanes remain available,
  - square profile keeps cavalry/skirmisher on `middle` by default, while diamond profile shifts cavalry/skirmisher preference to `outer`,
  - special-role overrides are formation-aware (`tracker`, `scout`, fanatic branch, priest/support branches) while preserving deterministic fallback ordering,
  - tie-breaking remains deterministic (`lane_priority_key`, then stable entity index ordering),
  - balance fixtures now lock support-heavy and cavalry-heavy placement envelopes to prevent lane-role drift across tier/hero roster growth.
- Added lane assignment debug overlay for QA:
  - `F8` toggles runtime lane inspector (`ON/OFF`) without changing gameplay simulation,
  - HUD row shows live lane totals (`O/M/I`),
  - friendlies receive per-unit lane labels (`O`, `M`, `I`) color-coded by lane.
- Formation roster and mechanics are now expanded:
  - live formations: `Square`, `Circle`, `Skean`, reworked `Diamond`, `Shield Wall`, `Loose`,
  - anti-entry is config-driven (`Diamond` + `Shield Wall` enforce hard `0` enemies inside),
  - `Loose` allows unlimited enemy interior occupancy (inside-cap repel disabled),
  - `Shield Wall` grants shielded block bonus and reflects melee-hit damage based on post-mitigation hit result.
- Formation QA usage windows (counterplay matrix) are locked by deterministic tests:
  - `Shield Wall` tops swarm/cavalry-heavy pressure profiles,
  - `Loose` tops ranged-heavy spread pressure profiles,
  - `Diamond` tops mixed pressure profiles with movement-centric engagement,
  - no single formation is allowed to dominate all tested pressure profiles.
- Anti-entry and reflect semantics are explicitly regression-covered:
  - anti-entry formations enforce zero interior occupancy caps,
  - Shield Wall reflect uses post-mitigation hit result, preserves source-hit critical impact in reflected amount, and does not emit reflected critical-hit text.
- Added `ArchivePlugin` + `ArchiveDataset` with generated codex entries (units/enemies/skills/stats/bonuses/drops).
- Added shared archive renderer used by both in-run `B` modal and main-menu `Bestiary` screen.
- Added mouse-wheel scrollable sections for `Archive`/`Bestiary` and `Skill Book` to prevent clipping.
- Stats table content in the in-run stats modal is now scrollable to prevent overflow.
- Main menu flow now exposes:
  - `Play Offline` (opens match setup)
  - `Play Online` (disabled placeholder)
  - `Settings`
  - `Bestiary`
  - `Exit`
- Added `MatchSetup` screen:
  - faction selection (`Christian` and `Muslim` both enabled),
  - difficulty selection (`Recruit`, `Experienced`, `Alone Against the Infidels`),
  - commander selection row per selected faction (multiple commanders per faction),
  - hover tooltip with commander description, base stats, run bonuses, and abilities,
  - map selection from data-driven map list,
  - start gate that only allows valid faction/difficulty/commander/map combinations.
- Added data-driven difficulty profile config in `assets/data/difficulties.json`:
  - per-difficulty enemy stat multipliers (health, damage, attack speed, move speed, morale),
  - per-difficulty behavior flags (`enemy_ranged_dodge_enabled`, `enemy_block_enabled`, `ranged_support_avoid_melee`).
- Difficulty behavior toggles are now live in runtime:
  - `Recruit`: no enemy projectile-dodge and no enemy block behavior,
  - `Experienced`: enemy projectile-dodge + enemy block behavior enabled,
  - `Alone Against the Infidels`: stronger enemy projectile-dodge/block plus ranged/support retreat spacing when melee pressure closes distance.
- Added roster level-budget economy:
  - tier-0 units cost `0` locked levels,
  - each tier-step promotion adds `+1` locked level,
  - unit death refunds the unit's locked level cost,
  - allowed max commander level is derived from locked budget (`100 - locked_levels`, saturating).
- Added progression/upgrade lock feedback surfaces:
  - `ProgressionLockFeedback` emits reason text when pending level rewards are blocked by roster costs,
  - `RosterEconomyFeedback` emits reason text when promotions are rejected by budget/path constraints,
  - Unit Upgrade modal now displays live budget and latest block reason strings.
- Replaced wave-number tier unlocks with major-army defeat unlocks:
  - tier 1 unlocks after defeating the major army on wave 10,
  - tier 2 after wave 20 major army,
  - tier 3 after wave 30 major army,
  - tier 4 after wave 40 major army,
  - tier 5 after wave 50 major army.
- Added `Hero` equipment-tier scaffold for inventory/UI; dedicated hero equipment effects are still pending.
- Replaced peasant-priest potion placeholder sprite with a character sprite (`tile_0109`) for clearer class readability.
- Implemented Unit Upgrade modal runtime:
  - left roster list with selectable unit source rows,
  - tier-column node graph (`Tier 0..5 + Hero`) with active tier-1, tier-2, tier-3, tier-4, and tier-5 promotion nodes,
  - tier-0 swap context menu (`right-click` source unit -> `Swap 1`) plus active hero subtype recruit nodes with unlock/token status text.
- Promotion validation now rejects non-upgrade paths (same-tier or invalid-tier conversions).
- Enemy wave runtime now uses layered army scheduling with hard wave lock:
  - `Small` army lane every wave,
  - `Minor` army lane every other wave,
  - `Major` army lane every 10th wave.
- Each wave still runs through staggered batch emission until all pending units are cleared.
- Next wave does not start until both pending batches and alive enemy count reach zero.
- Wave progression is finite at `100` waves; victory triggers only when wave-100 spawning is finished and all enemies are cleared.
- Enemy army progression now mirrors player major/minor cadence by level:
  - `major_count = floor(level / 5)`,
  - `minor_count = level - major_count`.
- Enemy army level assignment by difficulty:
  - `Recruit`: `floor(player_level / 2)` (min 1),
  - `Experienced` / `Alone Against the Infidels`: matches player level.
- Enemy army item pressure now comes from deterministic chest-template loadouts:
  - per-lane slot budgets (`Small=3`, `Minor=4`, `Major=5`),
  - per-difficulty fill ratios (`Recruit=1/3`, `Experienced=1/2`, `Infidels=2/3`),
  - per-difficulty rarity pressure modifiers (`0.0`, `0.18`, `0.33`),
  - generated item stats feed role-aware enemy damage/armor/health/speed scaling.
- Major army defeats now emit dedicated boss reward drops:
  - exactly two equipment chests per defeated major army wave,
  - chest positions are spread around the defeat location with non-overlap and map-bound clamps.
- HUD commander level text now renders `current/allowed` and appends a lock marker when progression is budget-locked.
- Rescue config now includes `recruit_pool` and validator rejects non-tier0 rescue entries.
- Added inventory scaffold module/resource (`InventoryState`) with serializable bag/equipment setup model.
- Inventory modal now renders:
  - bag drops as 1-item-per-slot grid,
  - separate equipment rows for commander and each unit tier (`Tier 0..5`) plus scaffolded `Hero` row.
- Commander slots: `Banner`, `Instrument`, `Chant`, `Squire`, `Symbol`; unit-tier slots: `Melee`, `Ranged`, `Armor`, `Banner`, `Squire`.
- Backpack viewport is now `5x6` slots.
- Stats modal now renders a table (`Stat | Base | Bonus | Final`) with color-coded bonuses (green positive, red negative).
- Skill Book modal now uses structured upgrade records with:
  - category grouping (formations/auras/combat/utility)
  - icon + description rows
  - stack-aware entries
  - formation active/inactive indicators
  - locked formation entries with explicit unlock requirement text (`Major` level-up rewards).
- Skill Book now displays cumulative effect totals per owned upgrade entry.
- Renamed the old recruit `Infantry/Knight` to `Christian Peasant Infantry`.
- Added `Christian Peasant Archer` as a second recruitable retinue unit.
- Rescue spawns now use a data-driven recruit pool and are currently constrained to tier-0 entries only.
- Active tier-0 rescue pool entries now use generic archetype IDs in `rescue.json` and resolve to the selected faction variant at runtime.
- Added rescuable-priest variant mapping so priest rescues flow through the same recruit pipeline.
- Recruit events now preserve rescued unit type so formation/combat/collision pipelines auto-handle both variants.
- Wired equipment bonuses into combat runtime:
  - melee-weapon slot `base_bonus` applies to melee damage only by default,
  - ranged-weapon slot `base_bonus` applies to ranged damage only by default,
  - armor slot `base_bonus` applies to armor only by default,
  - explicit per-item fields (`melee_damage_bonus`, `ranged_damage_bonus`, `armor_bonus`) add cross-slot effects when desired.
- Gear effects are tier-targeted:
  - commander setup affects commander only,
  - tier setups affect only units in that exact tier.
- Upgraded ranged combat to a shared unit system (no longer commander-only).
- Christian Peasant Archer now uses hybrid combat: weak melee profile + stronger projectile ranged profile.
- Added formation skillbar (bottom-center, 10 slots, keys `1..0`) with exclusive active formation switching.
- Square formation now uses neutral multipliers (`x1` baseline).
- Added one-time formation unlock cards in level-up draft (`Circle`, `Skean`, `Diamond`, `Shield Wall`, `Loose`):
  - unlock cards are skillbar-bound,
  - appear only on `Major` reward level-ups,
  - each appears once per run,
  - acquired formation unlocks auto-add to next free skillbar slot.
- Added simple generated formation icons for skillbar/cards:
  - `assets/sprites/skills/formation_square.png`
  - `assets/sprites/skills/formation_diamond.png`
- Added expanded formation tuning:
  - `Circle`: higher defense, lower mobility/offense,
  - `Skean`: faster movement and stronger moving offense with defense penalty,
  - reworked `Diamond`: moving-offense mobility profile + anti-entry,
  - `Shield Wall`: high defense, low speed, anti-entry, shielded block bonus, melee reflect,
  - `Loose`: wider spacing and unlimited enemy interior occupancy.
- Diamond/Skean slot assignment uses explicit ring + clockwise ordering around commander for clearer unit arrangement.
- Draft filtering now removes skillbar-bound cards when skillbar is full.
- Replaced the level-up pool with weighted random 5-option drafts from repeatable upgrades plus one-time skill unlocks.
- Upgrade values now roll via weighted min/max sampling (higher values are rarer).
- Aura upgrades (`Authority`, `Hospitalier`) now apply in runtime morale systems:
  - `Authority`: friendly morale-loss mitigation in aura + enemy morale drain in aura,
  - `Hospitalier`: morale regen in aura.
- Added shared ranged projectile attacks (outside-melee targeting, projectile travel, despawn on hit/max distance).
- Added gold pack minimap markers (yellow blips).
- Added commander movement slowdown from enemy pressure inside formation bounds (capped at 50% minimum speed multiplier).
- Pause menu button label now reads `Main Menu`.
- Added mandatory `LevelUp` state with 5-card draft overlay (image + description) and no skip path.
- Level-up weighted upgrades now roll into fixed 5-tier value buckets:
  - `Common`, `Uncommon`, `Rare`, `Epic`, `Mythical`
  - one-time upgrades (`formations`, `mob_*`) are classified as `Unique`
- Level-up card visuals now use tier-based border + glow colors:
  - `Common` = white/gray, `Uncommon` = blue, `Rare` = green,
    `Epic` = purple, `Mythical` = orange, `Unique` = red.
- Raised banner follow offset so it renders visibly behind/above the commander during movement.
- Dropped banner now uses the standard upright banner sprite for stronger in-world readability.
- Minimap now shows dropped-banner position and rescuable-retinue positions.
- Added one-time level-up upgrade `Into the Wolf's Dev` (`formation_breach`): once acquired, enemies inside the friendly formation footprint take `+20%` damage.
- Added critical-hit combat stats for friendlies:
  - `crit_chance_bonus` (additive chance, clamped to 95%)
  - `crit_damage_multiplier` (base `x1.2`, increased by upgrades)
- Added repeatable level-up cards:
  - `Killer Instinct` (`crit_chance`)
  - `Deadly Precision` (`crit_damage`)
- Added repeatable level-up cards:
  - `Master Quartermaster` (`item_rarity`) -> boosts equipment roll rarity.
  - `Tactical Insight` (`upgrade_rarity`) -> boosts upgrade value-roll rarity.
  - `Enduring Cadence` (`skill_duration`) -> boosts active-skill duration.
  - `Swift Drills` (`cooldown_reduction`) -> reduces active-skill cooldowns.
- Stats modal now shows `Crit Chance` and `Crit Damage` bonus rows.
- Skill Book cumulative descriptions now include crit chance and crit damage totals.
- Removed decorative floor foliage overlay; battlefield floor now renders as pure sand tiles only.
- Switched foliage overlay to transparent detail tile to remove opaque square artifacts on the floor.
- Enemy waves now spawn as staggered batches at pseudo-random positions across the playable map (not border ring-only).
- `Escape` now only triggers while in `InRun`, opening a centered pause overlay with `Resume`, `Restart`, and `Main Menu`.
- Added enemy chase hysteresis and removed unit position snapping to reduce movement jitter.
- Added delayed enemy gold drops (`0.9s` pickup lock) before homing can start.
- Ambient gold packs now spawn around commander position for better visibility.
- Gold homing speed now scales from commander base speed and stays slightly faster.
- Increased base drop pickup radius from `30` to `45`.
- Fixed Windows installer asset coverage for runtime-loaded art (`assets/sprites` + `oga_ishtar` pack).
- Switched battlefield floor to cleaner sand tile set.
- Added visible perimeter wall ring and hard playfield confinement for units.
- Added first minimap prototype (now top-right HUD panel) with commander/friendly/enemy blips.
- Enabled `CollisionPlugin` in app wiring (enemy collision now active).
- Added `GameOver` overlay flow with `Restart` and `Main Menu` actions.
- Rebuilt map floor rendering into tiled desert ground.
- Increased Christian Peasant Infantry attack range from `32` to `36`.
- Added drop transit-to-commander flow: friendly pickup starts homing, drop effect triggers only on commander contact.
- Added floating combat signal text:
  - `DamageTextEvent` emitted from finalized hit resolution,
  - world-space signal labels (`BLOCK!`, `CRITICAL HIT!`) with rise/fade animation,
  - per-frame and active-entity caps to prevent feedback spikes under high hit density.
- Reduced dense enemy crowd jitter/stacking:
  - enemy collision radius is now data-driven per enemy profile in `enemies.json`,
  - collision correction now uses frame-time-aware damping + max push clamp,
  - collision solver now runs 3 iterative passes per frame with per-pass push cap,
  - enemy-enemy pairs use larger separation distance (`x1.20`) to reduce mass overlap,
  - chase movement step is clamped to avoid overshooting into stop distance.
- Added per-upgrade requirement framework:
  - data schema now supports typed requirement discriminators (`tier0_share`, `formation_active`, `map_tag`, `has_trait`, `band_at_least`, `band_at_most`),
  - conditional upgrade ownership is tracked generically (not hardwired per-mob-flag),
  - conditional effects are re-evaluated continuously and cleanly revoke when requirements are unmet,
  - Skill Book now surfaces owned conditional upgrades as active/inactive with unmet-requirement messaging.
- Completed `Mob's Fury` + `Mob's Justice` runtime feedback loop:
  - `Mob's Fury` active/inactive state now appears in-run in the top-center HUD status line,
  - `Mob's Justice` execute hits no longer use dedicated floating damage text,
  - execute resolution uses a shared threshold helper (`<=10%` HP) across melee and projectile hit paths.
- Completed `Mob's Mercy` conditional rescue-speed effect:
  - rescue channel duration is computed via shared `effective_rescue_duration` and multiplied by conditional effects,
  - active Mercy state now appears alongside other mob upgrade statuses in the in-run HUD line,
  - added explicit tests for Mercy activation/deactivation and non-cross-wired interaction with Fury/Justice.
- Completed `Christian Peasant Priest` support runtime:
  - priest promotion now initializes full `20s` support cooldown and does not attach direct attack profiles,
  - priest outgoing damage is hard-blocked in combat runtime (they deal `0` damage even with global damage upgrades),
  - priests auto-cast a `10s` attack-speed blessing on friendlies in range and overlapping casts refresh duration,
  - in-run HUD status line now surfaces active priest blessing remaining time,
  - blessed friendlies render a subtle golden ground-shadow marker while the priest blessing is active.
- Replaced placeholder `morale_weight` usage with active per-unit `Morale` (friendlies and enemies).
- Morale runtime is now single-axis in active gameplay:
  - high bracket (`51..100`) grants gradual damage/armor/HP-regen bonuses,
  - low bracket (`<50`) applies armor penalty and escape-speed bonus,
  - encirclement pressure drains morale after a delay; no-pressure windows recover morale.
- Collapse loop at average morale `<= 0`:
  - drops 10% of retinue as rescuables (min 1),
  - resets morale after 3s to 70% with 6s grace.
- Reworked banner loop:
  - auto-drop at zero average morale
  - 10s pickup unlock delay
  - 5s pickup channel
  - dropped state disables commander banner-item effects
- Added HUD bottom-left vertical meter for average army morale plus threshold-crossing toast messages.
- Added banner pickup progress bar under treasury indicator.
- Removed oasis from active runtime schema/config usage.

## Unit Upgrade Tree Flowchart
Maintenance rule: update this diagram whenever promotion paths or tiers change.

```mermaid
flowchart LR
  PI["Peasant Infantry"] --> MA["Men-at-Arms"]
  PA["Peasant Archer"] --> BW["Bowman"]
  PP["Peasant Priest"] --> DV["Devoted"]

  MA --> SI["Shield Infantry"]
  MA --> SP["Spearman"]
  MA --> UK["Unmounted Knight"]
  MA --> SQ["Squire"]

  BW --> EB["Experienced Bowman"]
  BW --> XB["Crossbowman"]
  BW --> TR["Tracker"]
  BW --> SC["Scout"]

  DV --> DO["Devoted One"]
  DV --> FA["Fanatic"] 

  SI --> ESI["Experienced Shield Infantry"]
  SP --> SSP["Shielded Spearman"]
  UK --> KN["Knight"]
  SQ --> BN["Bannerman"]
  EB --> ELB["Elite Bowman"]
  XB --> AXB["Armored Crossbowman"]
  TR --> PF["Pathfinder"]
  SC --> MSC["Mounted Scout"]
  DO --> CAR["Cardinal"]
  FA --> FLA["Flagellant"]

  ESI --> ESH["Elite Shield Infantry"]
  SSP --> HAL["Halberdier"]
  KN --> HKN["Heavy Knight"]
  BN --> EBN["Elite Bannerman"]
  ELB --> LBM["Longbowman"]
  AXB --> EXB["Elite Crossbowman"]
  PF --> HM["Houndmaster"]
  MSC --> SHC["Shock Cavalry"]
  CAR --> ECAR["Elite Cardinal"]
  FLA --> EFLA["Elite Flagellant"]

  ESH --> CG["Citadel Guard"]
  HAL --> AHAL["Armored Halberdier"]
  HKN --> EHKN["Elite Heavy Knight"]
  EBN --> GC["God's Chosen"]
  LBM --> ELBM["Elite Longbowman"]
  EXB --> SXB["Siege Crossbowman"]
  HM --> EHM["Elite Houndmaster"]
  SHC --> ESHC["Elite Shock Cavalry"]
  ECAR --> DS["Divine Speaker"]
  EFLA --> DJ["Divine Judge"]

  HTU{{"Hero Tier Unlock\n(Wave 60 Major Defeat + Hear the Call)"}}
  HTU --> HSS["Hero: Sword+Shield"]
  HTU --> HSPH["Hero: Spear"]
  HTU --> H2H["Hero: 2H Sword"]
  HTU --> HBW["Hero: Bow"]
  HTU --> HJV["Hero: Javelin"]
  HTU --> HBM["Hero: Beast Master"]
  HTU --> HPR["Hero: Super Priest"]
  HTU --> HFAH["Hero: Super Fanatic"]
  HTU --> HSK["Hero: Super Knight"]
```

## Runtime Architecture

### App Builders
- Runtime app: `build_runtime_app()` in `src/lib.rs`
  - `DefaultPlugins`
  - Window: `1280x720`
- Headless app: `build_headless_app()`
  - `MinimalPlugins`

### Plugin Order (`configure_game_app`)
1. `DataPlugin`
2. `ArchivePlugin`
3. `CorePlugin`
4. `SettingsPlugin`
5. `PerformancePlugin`
6. `VisualPlugin`
7. `MapPlugin`
8. `InventoryPlugin`
9. `SquadPlugin`
10. `FormationPlugin`
11. `CollisionPlugin`
12. `RescuePlugin`
13. `DropsPlugin`
14. `EnemyPlugin`
15. `CombatPlugin`
16. `ProjectilePlugin`
17. `MoralePlugin`
18. `BannerPlugin`
19. `UpgradePlugin`
20. `UiPlugin`
21. `PlatformPlugin`

### Runtime Note
- `src/collision.rs` is now registered in app setup.
- `src/archive.rs` generates and validates bestiary/archive entries from loaded game data.

### Game States
- `Boot`
- `MainMenu`
- `MatchSetup`
- `Archive`
- `Settings`
- `InRun`
- `LevelUp` (run is paused until an upgrade card is selected)
- `Paused`
- `GameOver` (defeat pauses run and shows overlay actions)
- `Victory`

## Data Files and Live Values
Loaded from `assets/data` by `GameData::load_from_dir`.

### `units.json`
- Commanders:
  - `commander_christian` (`baldiun`)
  - `commander_muslim` (`saladin`)
- Recruit profiles (tier 0):
  - `recruit_christian_peasant_infantry`
  - `recruit_christian_peasant_archer` (hybrid melee+ranged)
  - `recruit_christian_peasant_priest` (non-damaging support)
  - `recruit_muslim_peasant_infantry`
  - `recruit_muslim_peasant_archer` (hybrid melee+ranged)
  - `recruit_muslim_peasant_priest` (non-damaging support)

### `roster_tuning.json`
- `tier2_units`:
  - per-tier2-kind `UnitStatsConfig` entries for both factions
  - consumed by promotion/loadout runtime for tier-2 stat setup
- `behavior`:
  - `tracker_hound_active_secs`
  - `tracker_hound_cooldown_secs`
  - `tracker_hound_strike_interval_secs`
  - `tracker_hound_damage_multiplier`
  - `scout_raid_active_secs`
  - `scout_raid_cooldown_secs`
  - `scout_raid_speed_multiplier`
  - `fanatic_life_leech_ratio`

### `enemies.json`
- Christian enemy profiles:
  - `enemy_christian_peasant_infantry`
  - `enemy_christian_peasant_archer`
  - `enemy_christian_peasant_priest`
- Muslim enemy profiles:
  - `enemy_muslim_peasant_infantry`
  - `enemy_muslim_peasant_archer`
  - `enemy_muslim_peasant_priest`
- Each profile includes: `max_hp`, `armor`, `damage`, `attack_cooldown_secs`, `attack_range`, optional ranged fields, `move_speed`, `morale`, `collision_radius`.

### `formations.json`
- `square`: `slot_spacing=30`, `offense=1.0`, `offense_while_moving=1.0`, `defense=1.0`, `anti_cavalry=1.0`, `move_speed=1.0`
- `circle`: `slot_spacing=30`, `offense=0.95`, `offense_while_moving=0.95`, `defense=1.12`, `anti_cavalry=1.02`, `move_speed=0.95`
- `skean`: `slot_spacing=30`, `offense=1.0`, `offense_while_moving=1.28`, `defense=0.82`, `anti_cavalry=0.92`, `move_speed=1.16`
- `diamond`: `slot_spacing=30`, `offense=1.0`, `offense_while_moving=1.22`, `defense=0.95`, `anti_cavalry=0.95`, `move_speed=1.14`, `anti_entry=true`
- `shield_wall`: `slot_spacing=30`, `offense=0.92`, `offense_while_moving=0.85`, `defense=1.2`, `anti_cavalry=1.1`, `move_speed=0.72`, `anti_entry=true`, `shielded_block_bonus=0.1`, `melee_reflect_ratio=0.3`
- `loose`: `slot_spacing=44`, `offense=1.04`, `offense_while_moving=1.04`, `defense=0.94`, `anti_cavalry=0.94`, `move_speed=1.08`, `allow_unlimited_enemy_inside=true`

### `waves.json`
Scripted waves:
1. `t=0`, `count=8`
2. `t=30`, `count=12`
3. `t=60`, `count=16`
4. `t=90`, `count=20`
5. `t=120`, `count=24`

Runtime conversion to spawn pacing:
- `wave_base_count`:
  - uses configured `count` while `wave_number` is inside scripted entries,
  - then continues from the last scripted count with `last_count * 1.18^(extra_waves)`.
- `units_per_second_for_wave = clamp(wave_base_count * 2.0, 1.0, 200.0) / 30.0`
- `wave_stat_multiplier = 1.0 + (wave - 1) * 0.102`
- Batch emission:
  - `batch_size = clamp(7 + wave/4, 7, 22)`
  - `batch_interval = clamp(0.7 - wave*0.01, 0.24, 0.7)`

### `drops.json`
- `initial_spawn_count=8`
- `spawn_interval_secs=2.5`
- `pickup_radius=45`
- `gold_per_pack=7`
- `max_active_packs=5000`

### `rescue.json`
- `spawn_count=6`
- `rescue_radius=60`
- `rescue_duration_secs=2.2`
- `recruit_pool` uses generic tier-0 archetype IDs (`peasant_infantry`, `peasant_archer`, `peasant_priest`); runtime resolves selected-faction variants and applies pity weighting.

### `upgrades.json`
- formation unlock cards (`one_time`, `adds_to_skillbar=true`):
  - `unlock_formation_circle`
  - `unlock_formation_skean`
  - `unlock_formation_diamond`
  - `unlock_formation_shield_wall`
  - `unlock_formation_loose`
- `encirclement_doctrine` (`kind=formation_breach`, `one_time`, grants inside-formation damage bonus)
- `damage`
- `attack_speed`
- `quartermaster` (repeatable gold gain multiplier for all gold packs)
- `armor`
- `pickup_radius`
- `aura_radius`
- `authority_aura`
- `move_speed`
- `hospitalier_aura`
- `luck` (repeatable shared luck layer: crit chance, crit damage, loot quality, and drop odds)
- `mob_fury` (`one_time`, `requirement_type=tier0_share`, `requirement_min_tier0_share=1.0`)
- `mob_justice` (`one_time`, `requirement_type=tier0_share`, `requirement_min_tier0_share=1.0`)
- `mob_mercy` (`one_time`, `requirement_type=tier0_share`, `requirement_min_tier0_share=1.0`)

Deterministic schema fields:
- `value` (authored fixed effect magnitude; no runtime roll tiers)
- `reward_lane` (`minor` | `major`)
- `one_time`
- `adds_to_skillbar`
- `formation_id`
- optional doctrine metadata:
  - `doctrine_tags`
  - `stack_cap`
  - `downside`
  - `major_unlock_hint`
- optional semantic-effect metadata (qualitative preview + trait hooks):
  - `effect_band_shift_stat` + `effect_band_shift_steps`
  - `effect_band_floor_stat` + `effect_band_floor_min`
  - `effect_trait_hook` + `effect_trait_modifier_kind` + `effect_trait_modifier_value`
- requirement fields:
  - `requirement_type`
  - `requirement_min_tier0_share`
  - `requirement_active_formation`
  - `requirement_map_tag`
  - `requirement_trait`
  - `requirement_band_stat`
  - `requirement_band_at_least`
  - `requirement_band_at_most`

### Upgrade Audit Disposition (`CRU-260`)
- Audit scope: all live upgrade IDs in `assets/data/upgrades.json`.
- `Keep`:
  - formation unlocks (`unlock_formation_circle`, `unlock_formation_skean`, `unlock_formation_diamond`, `unlock_formation_shield_wall`, `unlock_formation_loose`)
  - formation doctrine (`encirclement_doctrine`)
  - auras (`authority_aura_up`, `hospitalier_aura_up`)
  - tradeoff slot modifier (`war_council_edict`)
- `Merge/Rework`:
  - core stat stackers (`damage_up`, `armor_up`, `attack_speed_up`, `move_speed_up`, `cooldown_reduction_up`, `skill_duration_up`) should move to additive integer-point semantics with qualitative threshold messaging (`CRU-261`, `CRU-263`, `CRU-264`, `CRU-265`)
  - economy stackers are consolidated into one deterministic `quartermaster_up` schema with fixed authored effect + stack controls instead of parallel legacy IDs (`CRU-273`)
  - conditional doctrine variants are consolidated to canonical doctrine IDs (`mob_fury`, `mob_justice`, `mob_mercy`) and no longer duplicated by variant-ID suffixes (`CRU-273`)
- `Keep with Balance Pass`:
  - `luck_up` and pickup/aura radius utility cards stay in roster, but their value contrast and pacing remain under rebalance certification (`CRU-262`)
- `Deprecate` (enforced at load):
  - legacy duplicate IDs are hard-rejected by validators with replacement hints: `fast_learner_up`, `fast_learner_up_10`, `fast_learner_up_15`, `mob_fury_shielded_host`, `mob_justice_frontline_bias`, `mob_mercy_support_ceiling` (`CRU-274`)
- No unaudited high-impact upgrade remains in the active catalog.

### Major Upgrade Catalogue (`CRU-213`, `CRU-204`)
- Category matrix (design contract):
  - broad stat spikes,
  - unit-role doctrine bonuses,
  - commander doctrine bonuses,
  - formation/aura tactical actives,
  - temporary auto-cast style doctrine effects,
  - counter-doctrine responses (anti-ranged, anti-cavalry pressure).
- Live doctrine families and tradeoffs:
  - `doctrine_command_net` (`control`, commander/aura): larger aura command footprint + stronger enemy morale pressure in aura; downside reduces global damage output.
  - `doctrine_stalwart_oath` (`sustain`, commander/frontline): stronger armor and sustain profile; downside reduces army movement speed.
  - `doctrine_forced_march` (`tempo`, commander/mobility): stronger mobility and cadence pressure; downside reduces armor.
  - `doctrine_execution_rites` (`execute`, frontline): conditional execute breakpoint + conditional damage boost; downside slows rescue throughput while active.
  - `doctrine_countervolley` (`anti_ranged`, archer pressure): conditional ranged burst cadence; downside increases morale loss under pressure.
  - `doctrine_pike_hedgehog` (`cavalry_pressure`, anti-cavalry): conditional anti-cavalry pressure + morale stability under encirclement; downside reduces formation movement speed.
- Exposure and anti-degeneracy rules:
  - major rewards occur at every level divisible by `5` (20 major rewards by level `100`),
  - doctrine cards are one-time and must carry explicit `downside`,
  - role-specialized doctrines rely on requirement gates (`has_trait`, `band_at_least`) to enforce composition commitment and prevent universal best-in-slot picks.

### Minor Upgrade Catalogue (`CRU-214`)
- Category matrix (implementation-ready):
  - moderate stat increases (`damage_up`, `attack_speed_up`, `armor_up`, `move_speed_up`),
  - commander support/economy bonuses (`quartermaster_up`, `commander_aura_radius_up`, `pickup_radius_up`),
  - aura support modifiers (`authority_aura_up`, `hospitalier_aura_up`),
  - utility/consistency bonuses (`luck_up`, `skill_duration_up`, `cooldown_reduction_up`).
- Minor policy contract:
  - minors are repeatable support picks and remain lane-locked to `reward_lane=minor`,
  - effect magnitudes are deterministic (`value` only; no value-roll tiers),
  - stack-control metadata (`stack_cap`, doctrine tags) is supported in schema and reserved for CRU-205 balancing pass,
  - minors are intended to reinforce chosen major doctrine, not replace it.

### `map.json`
- Data-driven map list:
  - `id`, `name`, `description`
  - `width`, `height`
  - `allowed_factions`
  - optional `spawn_profile_id`
- Current runtime entry:
  - `desert_battlefield` (`2400x2400`, `allowed_factions=["christian","muslim"]`)

## ECS Inventory

### Core Components (`src/model.rs`)
- `Unit { team, kind, level }`
- `Health { current, max }`
- `BaseMaxHealth`
- `Morale { current, max }`
- `Armor`
- `AttackProfile`
- `AttackCooldown`
- `MoveSpeed`
- `ColliderRadius`
- Markers/data components: `PlayerControlled`, `FriendlyUnit`, `EnemyUnit`, `RescuableUnit { recruit_kind }`, `CommanderUnit`

### Module Components
- `RangedAttackProfile`, `RangedAttackCooldown` (`src/combat.rs`)
- `BanditVisualRuntime`, `BanditVisualState` (`src/enemies.rs`)
- `RescueProgress` (`src/rescue.rs`)
- `GoldPack`, `DropInTransitToCommander`, `MagnetPickup` (`src/drops.rs`)
- `Projectile` (`src/projectiles.rs`)
- `BannerMarker` (`src/banner.rs`)

### Resources
- `RunSession`
- `MatchSetupSelection`
- `RunModalState`
- `FrameRateCap`
- `GameData`
- `MapBounds`
- `InventoryState`
- `SquadRoster`
- `ActiveFormation`, `FormationModifiers`
- `FormationSkillBar`
- `WaveRuntime`
- `BannerState`, `BannerMovementPenalty`
- `Progression`, `UpgradeDraft`, `GlobalBuffs`
- `ProgressionLockFeedback`
- `OneTimeUpgradeTracker`
- `ConditionalUpgradeOwnership`
- `ConditionalUpgradeStatus`
- `RosterEconomy`, `RosterEconomyFeedback`
- `UnitUpgradeUiState`
- `CommanderMotionState`
- `HudSnapshot`
- `PlatformRuntime`
- `ArtAssets`

### Events
- `StartRunEvent`
- `RunModalRequestEvent`
- `RecruitEvent`
- `PromoteUnitsEvent`
- `DamageEvent`
- `UnitDamagedEvent`
- `DamageTextEvent`
- `UnitDiedEvent`
- `GainGoldEvent`
- `SpawnGoldPackEvent`

## Key Gameplay Formulas

### Morale Brackets + Movement (`src/morale.rs`, `src/squad.rs`, `src/enemies.rs`)
Core thresholds:
- neutral start: `0.51`
- low threshold: `0.50`

Bonus bracket (`51..100` morale):
- `morale_bonus_scale = clamp((ratio - 0.51) / (1.0 - 0.51), 0, 1)`
- damage multiplier bonus: up to `+8%`
- armor multiplier bonus: up to `+8%`
- HP regen bonus: up to `0.4% max HP/s`

Penalty bracket (`<50` morale):
- `morale_penalty_scale = clamp((0.50 - ratio) / 0.50, 0, 1)`
- armor penalty: up to `-12%`
- movement becomes escape-biased: up to `+16%` speed at `0` morale

Applied movement multiplier:
- commander movement speed
- enemy movement speed
- formation anchor movement inherits commander motion

### Friendly Outgoing Multiplier Floor
Friendly combined outgoing multiplier has lower clamp:
- minimum `0.55`

### Role Counter Matrix (`src/model.rs`, `src/combat.rs`, `src/projectiles.rs`)
Counter contract is tag-driven and faction-agnostic (all checks key off shared `unit_id`-derived tags):
- role tags:
  - `frontline`
  - `anti_cavalry`
  - `cavalry`
  - `anti_armor`
  - `skirmisher`
  - `support`
  - `hero_doctrine`
- armor classes:
  - `unarmored`
  - `light`
  - `armored`
  - `heavy`

Runtime counter hooks (damage multiplier, clamped to `0.65..1.45`):
- anti-cavalry attacker vs cavalry defender: `+0.35`
- cavalry attacker vs frontline defender: `+0.18`
- cavalry attacker vs anti-cavalry defender: `-0.24`
- anti-armor attacker vs armor class:
  - `heavy`: `+0.30`
  - `armored`: `+0.20`
  - `light`: `+0.05`
  - `unarmored`: `-0.10`
- skirmisher attacker vs support defender: `+0.15`
- archer attacker (non anti-cavalry) vs cavalry defender: `-0.10`
- support attacker vs frontline defender: `-0.12`

Representative specialization mapping:
- shield + knight lines: `frontline`
- spear/halberd lines: `anti_cavalry`
- scout/shock lines: `cavalry`
- crossbow/javelin lines: `anti_armor`
- tracker/scout/fanatic lines: `skirmisher`
- priest/bannerman/cardinal/speaker lines: `support`
- hero endpoint roster (`citadel_guard`, `armored_halberdier`, `elite_heavy_knight`, `elite_longbowman`, `siege_crossbowman`, `elite_houndmaster`, `divine_speaker`, `divine_judge`, `elite_shock_cavalry`): `hero_doctrine`

### Enemy-In-Formation Vulnerability Bonus (`src/combat.rs`)
- Base state: no inside-formation vulnerability bonus is active.
- After the one-time upgrade `Into the Wolf's Dev` is acquired, enemies inside the friendly formation footprint take multiplier `1.2` from friendly outgoing damage.
- Formation footprint is approximated from:
  - commander position
  - current recruit count
  - active formation slot spacing
- If commander has no recruits, bonus does not apply.

### Movement Slowdown From Enemies Inside Formation (`src/squad.rs`)
- Commander movement applies additional multiplier based on enemy count inside active formation footprint.
- Per-enemy slowdown: `0.04` (4%).
- Minimum multiplier clamp: `0.5` (commander cannot be fully stopped by this effect).
- Formula:
  - `multiplier = clamp(1.0 - enemy_count * 0.04, 0.5, 1.0)`

### Formation Footprint Occupancy Cap + Repel (`src/enemies.rs`)
- Enemy interior occupancy policy is formation-config driven:
  - default cap: `max_inside = floor(retinue_count / 4)`,
  - anti-entry formations (`Diamond`, `Shield Wall`) force `max_inside = 0`,
  - loose formation sets unlimited interior occupancy (overflow repel disabled).
- Overflow enemies are sorted deterministically by distance-to-commander and redirected toward the active formation perimeter projection (square/diamond/circle shape math).
- Repel movement is step-limited each frame for stability (`280 units/sec`).

### Formation Combat/Movement Effects
- Formation offense multiplier now has a moving-state modifier:
  - `effective_offense = offense_multiplier * offense_while_moving_multiplier` when commander is moving.
- Commander movement speed is multiplied by active formation move-speed multiplier.
- Friendly effective armor is multiplied by active formation defense multiplier.
- Shield Wall adds:
  - shielded-friendly block chance bonus,
  - melee-hit reflect based on post-mitigation incoming damage.

### Ranged Projectile Attacks (`src/combat.rs`, `src/projectiles.rs`)
- Units with `RangedAttackProfile` fire projectiles only when targets are outside melee range and inside ranged range.
- Current ranged units: commander + both faction archer variants (`Christian`/`Muslim` Peasant Archer).
- Projectile is non-instant and travels via velocity each frame.
- Projectile despawns on hit or when max travel distance is consumed.

### Commander Level Rewards (`src/upgrades.rs`, `src/enemies.rs`)
- Level-ups are no longer XP-threshold based.
- Wave completion emits `WaveCompletedEvent { wave_number }`.
- Rewards are queued as pending level-ups:
  - default: `+1` for each completed wave,
  - checkpoint bonus: wave `98` grants `+2`.
- Draft lane cadence is level-based:
  - each processed level-up opens one draft,
  - levels divisible by `5` are marked `Major`, all others are `Minor`.
- Shared parity helper is available in runtime:
  - `major_count = floor(level / 5)`,
  - `minor_count = level - major_count`.
- The level-up draft opens from queued rewards while commander level is still under roster-allowed cap.

### Major/Minor Revamp Contract (`CRU-200`)
- Lane taxonomy:
  - `Minor`: support/pacing lane (frequent additive picks),
  - `Major`: milestone doctrine lane (high-impact picks with explicit tradeoff).
- Cadence:
  - one draft per level reward,
  - levels divisible by `5` route to `Major`; all other levels route to `Minor`,
  - deterministic progression target remains level `100` by end of wave `98`.
- Impact and tradeoff floor:
  - every major card must include an explicit downside/opportunity-cost hook,
  - every major card must produce at least one strategic breakpoint shift (band shift, trait/badge activation, formation/ability behavior change, or equivalent).
- Schema invariants:
  - upgrade strength is authored and deterministic (no runtime value-roll power resolution),
  - upgrade rarity can influence option appearance but not rolled power,
  - deterministic item templates own authored stat packages/tradeoffs; drop RNG selects template/source only.

### Commander Allowed Max Level from Roster Budget (`src/squad.rs`)
- Hard commander cap: `100`.
- Roster lock rule:
  - `allowed_max_level = saturating_sub(100, locked_levels)`
- Promotion guard:
  - a promotion is rejected if it would reduce `allowed_max_level` below current commander level.

### Unit Upgrade Promotion Affordability (`src/ui.rs`)
- For each tier-1 promotion node, UI computes:
  - `step_cost = promotion_step_cost(from_kind, to_kind)` (currently only valid tier-0 -> tier-1 branch links),
  - `gold_cost = promotion_gold_cost(step_cost, target_tier)`,
  - `next_locked = locked_levels + step_cost`.
- Promotion button is enabled only when all are true:
  - source unit count > `0`,
  - target tier is unlocked by major-army defeat gate,
  - `current_gold >= gold_cost`,
  - `level_cap_from_locked_budget(next_locked) >= commander_level`.
- Tooltip still reports `max_affordable` count for the same source row, but runtime action is currently a single-step `+1` promote.

### Conditional Upgrade Requirement Evaluation (`src/upgrades.rs`)
- Owned conditional upgrades are evaluated each frame from live runtime context:
  - `tier0_share` compares roster tier-0 ratio against configured minimum.
  - `formation_active` checks currently active formation id.
  - `has_trait` checks for minimum roster presence of trait families (`shielded`, `frontline`, `anti_cavalry`, `cavalry`, `anti_armor`, `skirmisher`, `support`).
  - `band_at_least` / `band_at_most` compares roster share stats (`tier0`, `shielded`, `frontline`, `anti_cavalry`, `support`, `cavalry`, `archer`, `anti_armor`) against qualitative bands (`very_low`..`very_high`).
  - `map_tag` is schema-supported; currently reports unmet in runtime until map tags are introduced.
- Effects are rebuilt from scratch each refresh and deduplicated by upgrade kind, preventing multi-variant stacking exploits of the same conditional family.

### Wave Spawn Rate + Victory Gate (`src/enemies.rs`, `src/core.rs`)
- Wave duration: `30s`.
- Spawn pacing:
  - `units_per_second_for_wave = clamp(wave_base_count * 2.0, 1.0, 200.0) / 30.0`
  - spawned units are queued into timed batches (`batch_size` scales by wave, interval shrinks with floor clamp).
- Enemy stat progression:
  - `wave_stat_multiplier = 1.0 + (wave - 1) * 0.102`.
- Wave progression:
  - `current_wave` increases until `MAX_WAVES = 100`.
  - spawning stops after wave 100 finishes its duration window.
- Victory condition:
  - `finished_spawning == true`
  - `current_wave >= 100`
  - `pending_batches` empty
  - alive enemy count is `0`

### Upgrade Draft Value Contract (`src/upgrades.rs`)
- Draft picks `5` unique upgrades from the configured pool.
- Draft lane routing is explicit:
  - `Minor` rewards draw only `reward_lane=minor`,
  - `Major` rewards draw only `reward_lane=major`.
- Upgrade value is deterministic:
  - chosen card uses authored `value` directly,
  - legacy roll-tier fields (`min_value/max_value/value_step/weight_exponent`) are rejected at load time.
- Minor repeatables support authored stack policies:
  - `stack_cap` removes capped cards from future draft pools,
  - `diminishing_factor` scales each subsequent stack value (`value * factor^current_stacks`).

### Morale Pressure + Collapse (`src/morale.rs`)
- Encirclement pressure:
  - pressure ratio is based on enemies inside formation footprint vs retinue size.
  - a 3s delay gate must be crossed before drain starts.
  - drain rate:
    `-1.1 * pressure_ratio * conditional_loss_multiplier * faction_loss_multiplier * authority_loss_multiplier * gear_loss_multiplier`.
  - chant immunity (`Battle Song`) sets morale-loss multiplier to `0` while active.
  - passive morale regen from commander equipment is always applied.
  - hospitalier aura morale regen is applied while inside commander aura.
  - when pressure is zero, baseline morale recovery is `+0.30 * faction_gain_multiplier` per second.
- Authority aura enemy pressure:
  - enemies inside commander aura lose morale each frame using:
    `authority_enemy_morale_drain_per_sec * faction_authority_multiplier * (1 + aura_enemy_effect_bonus_multiplier)`.
- Collapse trigger:
  - uses average friendly morale and triggers when average `<= 0` and grace is inactive.
  - removes `ceil(retinue * 0.10)` units (min 1), converting valid recruit kinds into rescuables.
  - reset is delayed by `3s`, then all friendly morale is restored to `70%`.
  - post-reset grace window: `6s`.
- Morale threshold events:
  - crossing edges: `25%`, `50%`, `80%`, `100%`,
  - events are emitted in edge order for both rising and falling transitions.

## Banner Loop (`src/banner.rs`)
- Auto-drop trigger: average friendly morale ratio `<= 0` (with anti-redrop grace check)
- Dropped effect: commander `Banner` item bonuses are disabled while banner is down
- Banner follow render offset: banner is rendered with positive Y offset behind commander for visibility.
- Pickup unlock delay: 10s after drop
- Pickup channel: 5s while friendly unit is within recovery radius
- Successful recovery:
  - banner returns to up state
  - redrop grace timer starts (10s)

### Banner Progress UI
- Banner channel progress is surfaced under treasury indicator through same progress-strip region used by rescue bars.

## Drop Flow (`src/drops.rs`)
1. Spawn ambient packs + event packs (enemy death events).
2. Enemy-death drops spawn with `0.9s` pickup delay before any homing can start.
3. Any friendly within pickup radius marks pack as `DropInTransitToCommander` (after delay).
   - Effective pickup radius = `base pickup radius + stacked pickup-radius upgrades`.
4. Transit pack homes to commander each frame at speed slightly above commander base speed.
5. On commander contact radius, pack is consumed and effect is applied (`GainGoldEvent`).
6. Magnet pickup lifecycle:
   - spawns at wave start on waves divisible by `3` (map center),
   - despawns automatically when the next wave starts.
7. Magnet pickup effect:
   - on friendly pickup, all active gold packs are immediately forced into transit-to-commander mode.
8. Equipment chest lifecycle:
   - one chest can spawn on wave transitions divisible by `3`,
   - chest pickup has `0.9s` unlock + `2.0s` channel,
   - successful channel opens the in-run `Chest` modal with `1..3` rolled items.
9. Major army chest rewards:
   - defeating a major army wave emits two additional chest drops,
   - duplicate rewards for the same major wave are blocked.

## System Summary (By Module)

### `core.rs`
- Boot -> menu transition
- Main menu cleanup
- menu clear color handling for `MainMenu`, `MatchSetup`, `Settings`, `Archive`
- in-run modal hotkeys (`I/O/K/B/U`) through reducer-based modal request flow
- `Escape` behavior priority:
  - close open run modal
  - otherwise open pause menu
- while paused, `Escape` resumes run (same as pause menu `Resume`)
- virtual time pause/unpause sync while run modal is open
- survival timer
- commander-loss transition to `GameOver`

### `map.rs`
- camera spawn
- map bounds init from selected match-setup map (fallback to first configured map)
- tiled desert floor spawn (respawned on run start to match selected map bounds)
- perimeter wall visuals
- camera follow + camera-only pixel snap + map-edge clamp
- unit confinement to playable area inside wall inset

### `inventory.rs`
- runtime inventory scaffold resource initialization
- unit-tier + commander equipment setup defaults
- serializable bag/equipment model
- deterministic item template pipeline from `assets/data/items.json`:
  - fixed per-template stat packages (`kind`, `value`, `rarity`)
  - explicit `downside`, `doctrine_tags`, and `nature` (`Minor`/`Major`) metadata
  - chest RNG selects template only; item stat values are not rarity-rolled at runtime
- slot-aware gear bonus resolution (`gear_bonuses_for_unit`)
  - default slot behavior applies `base_bonus` to matching stat only
  - explicit item bonus fields can add cross-stat effects

### `archive.rs`
- builds `ArchiveDataset` entries from live data files
- validates archive entries for required fields (title + description)
- exposes category groupings reused by main-menu and in-run archive UIs

### `collision.rs`
- resolves eligible overlap pairs (enemy-enemy and enemy-inner-retinue)
- applies iterative damped/clamped correction vectors for frame-rate-stable separation
- inflates enemy-enemy minimum spacing to reduce dense crowd overlap
- keeps post-separation positions inside map bounds

### `squad.rs`
- run start commander spawn
- commander movement (includes enemy-inside-formation slowdown multiplier)
- recruit spawn from rescue/upgrade events
- roster sync/casualties

### `formation.rs`
- square offsets and smoothing
- depth sorting
- formation movement is wired through `BannerMovementPenalty` (currently neutral `1.0` multiplier)

### `rescue.rs`
- start spawn + timed respawn of rescuables (`20s` cadence, max `6` active at once)
- typed rescuable metadata driven by `rescue.recruit_pool` (tier-0-only entries accepted by config validator)
- any-friendly rescue channel logic
- pity-weighted recruit-kind selection for rescue spawns (`weight = 1 + drought`) to reduce long spawn streaks of one type

### `drops.rs`
- ambient and event gold pack spawning with wave/level gold scaling
- pickup-delay-aware pack pickup detection (any friendly can trigger)
- transit-to-commander homing consume flow
- final gold award applies `GlobalBuffs.gold_gain_multiplier` at consume time (affects ambient + enemy-drop packs)
- wave magnet pickup spawn/despawn cadence (every 3 waves)
- magnet pickup force-homes all active gold packs
- major-army reward chest spawning (2 chests per major-wave defeat with dedupe + spread positioning)

### `enemies.rs`
- finite 100-wave runtime with units-per-second spawning
- queued enemy batch spawning with wave-scaled batch sizes/intervals
- no wave overflow: next wave starts only after current wave spawn queue is empty and all alive enemies are cleared
- pseudo-random spawn points within playable map bounds
- chase AI (retinue-prioritized targeting)
- difficulty-gated ranged/support melee-avoidance behavior (`ranged_support_avoid_melee`)
- active-formation inside-footprint cap with perimeter repel for overflow enemies
- visual state texture mapping

### `combat.rs`
- attack cooldown tick
- shared unit ranged projectile emission (commander + archer hybrid behavior)
- in-range targeting + damage emit
- melee outgoing base damage includes resolved equipment melee bonus
- ranged outgoing base damage includes resolved equipment ranged bonus
- friendly armor mitigation includes resolved equipment armor bonus
- enemy-in-formation vulnerability check (`+20%` friendly damage when inside formation bounds)
- friendly crit roll on melee and ranged outgoing hits (before armor mitigation)
- difficulty-gated enemy block behavior in hit resolution (`enemy_block_enabled`)
  - block rolls now require the target to have the explicit `Shielded` trait.
- damage apply + `UnitDamagedEvent` + `DamageTextEvent` (uses final applied damage, not requested pre-clamp amount)
- death resolve + drop spawn events

### `projectiles.rs`
- projectile travel + despawn-on-hit/max-distance
- projectile collision damage resolution with armor/morale modifiers
- difficulty-gated enemy ranged dodge checks for friendly projectiles (`enemy_ranged_dodge_enabled`)

### `morale.rs`
- run-start morale runtime reset (pressure/collapse/threshold trackers)
- encirclement-driven morale drain with delay + safe recovery while unpressured
- high-morale bonus / low-morale penalty bracket math
- collapse handling (retinue losses + delayed morale reset + grace window)
- threshold-crossing event emission (`25/50/80/100`)

### `banner.rs`
- run-start banner reset
- zero-morale drop trigger
- delayed pickup channel
- movement-penalty resource refresh (currently neutral)

### `ui.rs`
- main menu buttons (`Play Offline`, `Play Online` disabled, `Settings`, `Bestiary`, `Exit`)
- main-menu `Bestiary` screen (same dataset/content source as in-run archive modal)
- `MatchSetup` screen with faction + commander + map selectors and `Start`/`Back` actions
- commander hover tooltip in match setup (description, stats, abilities, run bonuses)
- settings screen with FPS selector
- global UI scale sync from live window resolution (`UiScale`) for resolution-mode resilience
- pause overlay buttons (`Resume`, `Restart`, `Main Menu`)
- level-up overlay (5 mandatory upgrade cards, icon + description, no skip)
- game-over overlay buttons (`Restart`, `Main Menu`)
- top HUD (left column: wave/time, center: level/treasury/rescue bars)
- progress strips (rescue + banner pickup)
- bottom-left vertical bar (average morale)
- morale-threshold toast text below top-center bars
- commander aura footprint indicator (subtle world-space circle around commander)
- world-space health bars with snapped segmented fill (threshold-crossing updates)
- world-space floating combat signal text with timed rise/fade cleanup
- top-right minimap prototype with periodic blip refresh (`204px`, +20% from previous size)
  - commander/friendlies/enemies
  - gold packs (yellow)
  - wave magnet pickup symbol (cross for Christian, crescent for Muslim)
  - rescuable retinue markers
  - dropped-banner marker
- utility action bar moved to bottom-right (swapped with minimap position)
- bottom-center skillbar (10 slots)
  - slot `1` default Square formation (active)
  - key labels `1..0`
  - active slot border highlight
- in-run modal overlay scaffolds for:
  - `Inventory`
  - `Stats`
  - `Skill Book`
  - `Bestiary`
  - `Unit Upgrade`
- inventory right-click context menu on backpack items:
  - `Equip` (same target resolution contract as double-click equip path)
  - `Scrap` (remove item and convert to gold using authored deterministic `scrap_gold_value` from the item template)
- inventory modal content:
  - bag drops grid (1 item = 1 slot, with empty placeholders)
  - fixed 5x6 backpack viewport (first 30 slots shown in-grid)
  - equipment panel with commander + unit tier rows using short labels (`C`, `T0..T5`, `H`)
- commander slots: `Banner`, `Instrument`, `Chant`, `Squire`, `Symbol`
  - unit-tier slots: `Melee`, `Ranged`, `Armor`, `Banner`, `Squire`
- stats modal content:
  - table layout (`Stat | Base | Bonus | Final`)
  - table rows are rendered in a scrollable viewport
  - `Unit HP` row is bonus-only (`Base` and `Final` show `-`)
  - bonus color coding (green positive, red negative)
  - separate `Active Buffs` column for formation/auras/conditional effects/priest blessing
- skill book modal content:
  - grouped sections (`Formations`, `Auras`, `Combat`, `Utility`)
  - icon-backed entries with stacked counts
  - cumulative effect descriptions per owned upgrade
  - active/inactive markers for mutually exclusive formation skills
  - formation entries include explicit `Strengths`, `Weaknesses`, and `Use case` tradeoff text
  - active/inactive markers + unmet-requirement detail for owned conditional upgrades
- archive/bestiary modal content:
  - mouse-wheel scrolling with clipped viewport to prevent content overflow
- top-right utility icon bar:
  - `Inventory` (`I`)
  - `Stats` (`O`)
  - `Skill Book` (`K`)
  - `Bestiary` (`B`)
  - `Unit Upgrade` (`U`)

### `upgrades.rs`
- Wave-reward level queue and explicit level-up draft flow (`InRun -> LevelUp -> InRun`)
- 5-option upgrade draft cards (keyboard `1..5` and mouse click selection)
- deterministic authored upgrade values (no runtime value-roll tiers)
- additive stacked upgrade effects with authored `stack_cap` + `diminishing_factor` support for repeatables
- repeatable `quartermaster` upgrade adds to `GlobalBuffs.gold_gain_multiplier`
- repeatable crit upgrades (`crit_chance`, `crit_damage`) wired into `GlobalBuffs`
- repeatable item-rarity upgrade (`item_rarity`) feeds drop-layer luck signals; item stat packages remain deterministic once a template is selected
- shared skill timing buffs:
  - `skill_duration` increases duration of cooldown-based skills
  - `cooldown_reduction` reduces cooldown of cooldown-based skills
- passive commander level scaling
- level-up full-heal sync for friendlies
- generic conditional-upgrade ownership + typed requirement parsing/evaluation
- runtime conditional status snapshot used by Skill Book UI

### `steam.rs`
- feature-gated platform runtime (`standalone`/`steam`)

## Current Hooks / Known Gaps
- `FormationModifiers.defense_multiplier` and anti-cavalry values are still not fully wired into incoming damage resolution.
