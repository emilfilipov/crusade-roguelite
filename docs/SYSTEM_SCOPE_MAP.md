# SYSTEM_SCOPE_MAP.md

## Purpose
This file defines what is intentionally limited in MVP and what is expected to expand later.
It prevents scope creep and makes expansion decisions explicit.

## MVP Core Loop (Scaffold)
1. Start run as commander (`Baldiun`) only, in square formation.
2. Fight basic `bandit_raider` enemies with commander auto-attack.
3. Find neutral rescuable soldiers on the map.
4. Keep any squad member (commander or retinue) near a rescuable soldier for the rescue channel duration.
5. On successful rescue, soldier joins the retinue and fights automatically.
6. Survive escalating waves while managing cohesion and banner state.
7. Draft upgrades on level-up.
8. Continue until squad collapse or defeat.

## Limited Now vs Expand Later
| System | MVP Limit (Now) | Planned Expansion (Later) | Expansion Gate |
| --- | --- | --- | --- |
| Formation Set | 1 formation: `Square` only | Add `Line`, `Wedge`, `Column` | Square formation stable across full runs with no slot/positioning bugs |
| Commander Kit | 1 passive aura + 1 battle cry + basic auto-attack | Multiple aura trees, command actives, commander loadouts | Commander-only early game is reliable and balanced |
| Recruit Roster | 1 recruit archetype/subtype: `Infantry/Knight` | Spearman, Archer, support units, faction variants | Rescue loop stable and roster growth bug-free |
| Enemy Roster | 1 enemy type: `bandit_raider` (melee infantry profile) | Archers, cavalry, elites, event waves | Basic wave pacing and melee combat deterministic |
| Enemy AI | 1 behavior profile: chase + melee | Ranged kiting, cavalry charge, coordinated groups | No AI deadlocks and target selection stable |
| Enemy Visual States | 1 state set for `bandit_raider`: idle/move/attack/hit/dead sprite swaps | Multi-frame animation sets and per-faction visual variants | State mapping remains readable and stable under combat load |
| Weapon Types | 1 weapon class: melee | Bows, spears, anti-armor, weapon traits | Melee combat formula validated by tests |
| Armor Model | 1 armor stat (flat mitigation) | Armor slots, resist types, durability | Damage model stable and readable |
| Skill System | Minimal per-unit fields only | Per-unit skill trees and triggered abilities | Unit lifecycle and leveling stable |
| Unit Progression | 1 progression track (simple level/veterancy) | Multi-stat growth profiles and role specialization | Level-up logic deterministic and test-covered |
| Rescue System | 1 rescue interaction: any friendly unit in radius for fixed time | Escort rescues, contested rescues, rescue events | Base rescue flow has no stuck/interruption bugs |
| Upgrade Draft | Small pool (6-10 options) | Wider pools, synergies, rarity tiers | Starter upgrade set balanced and test-covered |
| Morale/Cohesion | 1 cohesion meter with 2 thresholds | More morale states, event modifiers, trait interactions | Current thresholds produce clear tactical outcomes |
| Banner | 2 states: `Up` / `Dropped`, 1 recovery interaction | Banner traits, relocation rules, enemy banner threats | Drop/recover flow stable in combat stress |
| Map Set | 1 map: desert battlefield | Additional biomes/maps and tactical terrain variants | First map supports full run loop cleanly |
| Points of Interest | 1 POI type: oasis heal zone | Shrines, supply carts, event zones | Oasis interaction stable and not abusable |
| Resource Economy | 1 run currency: XP only (from kills + XP pickup drops) | Gold/supplies/reputation meta layers | XP and upgrade cadence balanced |
| UI | Minimal tactical HUD only | Advanced overlays, breakdown panels, analytics | Core HUD readable during heavy combat |
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
