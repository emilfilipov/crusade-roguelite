# SYSTEM_SCOPE_MAP.md

## Purpose
This file defines what is intentionally limited in MVP and what is expected to expand later.
It prevents scope creep and makes expansion decisions explicit.

## MVP Core Loop (Scaffold)
1. Start run as selected faction commander (`Baldiun` or `Saladin`) only, in square formation.
2. Fight faction-opposed enemy retinues (Christian vs Muslim pools) with commander auto-attack.
3. Find neutral rescuable soldiers on the map.
4. Keep any squad member (commander or retinue) near a rescuable soldier for the rescue channel duration.
5. On successful rescue, soldier joins the retinue and fights automatically.
6. Survive escalating waves while managing morale and banner state.
7. Draft upgrades on level-up.
8. Continue until squad collapse or defeat.

## Limited Now vs Expand Later
| System | MVP Limit (Now) | Planned Expansion (Later) | Expansion Gate |
| --- | --- | --- | --- |
| Formation Set | 2 formations: `Square` (neutral baseline) + `Diamond` (offense-while-moving, speed up, defense down); slot assignment currently prioritizes melee on outer rings and ranged/support toward inner rings | Add `Line`, `Wedge`, `Column` | Square+Diamond switching stable across full runs with no slot/positioning bugs |
| Commander Kit | Basic melee auto-attack + conditional ranged arrow attack; aura upgrades (`Authority`, `Hospitalier`) active via level-ups | Multiple aura trees, command actives, commander loadouts | Commander-only early game is reliable and balanced |
| Recruit Roster | 6 tier-0 recruit variants across 2 factions plus live tier-1, tier-2, tier-3, and tier-4 promotion branches (`Men-at-Arms/Bowman/Devoted` into tier-2 role branches, tier-3 continuations, then tier-4 continuations: elite shield infantry, halberdier, heavy knight, elite bannerman, longbowman, elite crossbowman, houndmaster, shock cavalry, elite cardinal, elite flagellant per faction) | Tier-5 capstones + hero-tier integration | Rescue/promotion loop stable and roster growth bug-free |
| Roster Level Budget | Tier-0 units cost `0`; each allowed promotion step adds `+1` locked level (including tier-preserving specialization paths); allowed max commander level = `100 - locked_levels` (saturating); unit death refunds its lock | Additional unit-economy dimensions (supply, upkeep, faction doctrine) | Current level-lock loop is readable, test-covered, and does not deadlock run progression |
| Enemy Roster | Opposite-faction enemy pool scaffolded to 3 tier-0 unit kinds (`Infantry`, `Archer`, `Priest`) per faction; player faction determines enemy faction pool | Archers/cavalry elites with dedicated AI and faction doctrines | Basic wave pacing and melee combat deterministic |
| Enemy AI | 1 core behavior family: chase-driven pressure with role-aware engage ranges (melee closes, ranged/support hold range/follow) and crowd-hold anti-jitter logic | Ranged kiting doctrines, cavalry charge, coordinated flanks | No AI deadlocks and target selection stable |
| Wave Runtime | Layered army lanes with hard wave lock: `Small` every wave, `Minor` every other wave, `Major` every 10th wave; staggered batch emission remains active; capped at 100 waves; next wave starts only after all enemies are defeated; major-army defeats emit a dedicated dual-chest reward event | Dynamic event waves, richer boss scripts, spawn directors | Layer cadence and no-overflow wave gating remain deterministic under stress |
| Enemy Visual States | Single shared state machine with faction-specific base sprites (Christian/Muslim enemy representations) | Full multi-frame animation sets and richer per-faction visual variants | State mapping remains readable and stable under combat load |
| Weapon Types | 2 baseline classes in use: melee + projectile ranged (`Commander` + `Christian Peasant Archer`) | Additional ranged classes, spears, anti-armor traits | Current melee+ranged interaction remains stable and readable |
| Armor Model | 1 armor stat (flat mitigation) | Armor slots, resist types, durability | Damage model stable and readable |
| Skill System | Core per-unit morale + commander aura buff scaffolding (`Authority`, `Hospitalier`) wired through upgrade/buff surfaces | Per-unit skill trees, triggered abilities, active aura skills | Base morale/aura interactions stable and test-covered |
| Unit Progression | 1 progression track (simple level/veterancy) | Multi-stat growth profiles and role specialization | Level-up logic deterministic and test-covered |
| Rescue System | 1 rescue interaction: any friendly unit in radius for fixed time; data-driven `recruit_pool` currently restricted to tier-0 units across both factions (Christian + Muslim peasant infantry/archer/priest) and filtered by selected player faction at runtime | Escort rescues, contested rescues, rescue events, higher-tier or map-specific rescue pools | Base rescue flow has no stuck/interruption bugs |
| Upgrade Draft | Mixed repeatable + one-time entries; random 5 choices; one-time cards are removed after pick; per-upgrade requirement schema (`tier0_share`, `formation_active`, `map_tag`) drives conditional runtime activation/deactivation with in-run status visibility | Wider pools, synergy bundles, rarity tiers | Starter upgrade set balanced and test-covered |
| Unit Upgrade Screen | In-run modal (`U`) with roster source selection, tier-column node graph, tier-0 right-click swap menu, active tier-1 promotions, active tier-2 branch buttons, active tier-3 continuation buttons, and active tier-4 continuation buttons; all promotions are constrained by unlock, treasury, source count, and level-budget affordability; tier milestones are major-army-gated (`T1@boss10`, `T2@boss20`, `T3@boss30`, `T4@boss40`, `T5@boss50`) | Tier-5 interactive branching, hero-node recruitment actions, richer previews | Current promotion economy + boss-gated unlock checks remain deterministic and test-covered |
| Skillbar | 10-slot bottom-center bar; slot `1` starts with `Square`; active skills use keys `1..0` | Click activation, drag-reorder, cooldown overlays | Keyboard activation + slot assignment stable and test-covered |
| Morale | Per-unit morale as the active discipline axis; high morale grants gradual combat/sustain bonuses, low morale adds armor penalties + escape-speed behavior, and `0` morale triggers timed retinue-collapse pressure/recovery | Richer morale events, panic/rout behavior, faction morale traits | Current morale loop remains recoverable and readable |
| Banner | Auto-drop when average friendly morale reaches `0`, then timed recovery channel with unlock/grace windows; dropped banner currently disables commander banner item effects | Banner traits, relocation rules, enemy banner threats | Auto-drop/recovery loop stable and not abusable |
| Map Set | Data-driven map list scaffold with 1 playable entry (`desert_battlefield`); selected from offline match setup before run start | Additional biomes/maps and tactical terrain variants | First map supports full run loop cleanly |
| Player Factions | Match setup exposes `Christian` and `Muslim` as playable factions; selected faction determines commander/recruit pool and opposite-faction enemy pool; faction identity is tuned through `assets/data/factions.json` (friendly stats, enemy-side stats, morale dynamics, aura tuning fields, rescue speed, gold gain) | Additional faction-exclusive mechanics, progression paths, and content depth | Dual-faction parity remains stable under full-run tests |
| Difficulty Profiles | Match setup exposes `Recruit`, `Experienced`, and `Alone Against the Infidels`; selected difficulty drives enemy stat multipliers through `assets/data/difficulties.json`, enemy block toggles, enemy ranged-dodge checks, and ranged/support melee-avoidance spacing behavior | Full strategy-package differentiation per difficulty (doctrine selection, active-skill behavior weighting, richer tactics) | Difficulty selection persists into run state and enemy scaling/behavior remains deterministic |
| Points of Interest | No active POI in runtime (oasis is deferred/config-only) | Oasis return, shrines, supply carts, event zones | Core loop stable before reintroducing POI interactions |
| Resource Economy | 1 run currency: Gold via collectible gold packs (ambient + enemy drop spawns); plus inventory scrapping for immediate gold conversion | Gold/supplies/reputation meta layers | Gold income and upgrade cadence balanced |
| UI | Tactical HUD with wave/level/time, rescue+banner/chest progress strips, bottom-left morale meter, and morale-threshold toasts | Advanced overlays, breakdown panels, analytics | Core HUD readable during heavy combat |
| Audio/FX | Placeholder-first minimal effects | Layered soundscape and richer combat FX | Gameplay readability preserved with effects enabled |

## Retinue Size Policy
- Design intent: no hard cap on retinue size.
- Implementation note: temporary soft caps are allowed only for performance safety and must be documented.
- Any temporary cap must include a follow-up task to remove or raise it.

## Expansion Checklist (Must Pass Before Adding More Variants)
1. Existing single-example implementation passes unit tests and gameplay smoke tests.
2. No open critical bugs in that system.
3. Determinism is acceptable under fixed timestep for affected logic.
4. Performance remains within target for expected encounter sizes.
5. `docs/TASKS.md` includes explicit tasks for the expansion work.
6. This file is updated in the same change that introduces expansion.
