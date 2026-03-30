# EXPANSION_QA_MATRIX.md

## Purpose
Integrated certification checklist for army/tier/hero expansion systems (`CRU-234`).

## Automated Regression Coverage
- Wave cadence + lane layering:
  - `enemies::tests::wave_batch_planner_layers_small_minor_and_major_armies`
  - `enemies::tests::wave_tier_mix_ramps_between_unlock_milestones`
  - `enemies::tests::major_wave_tier_mix_previews_next_tier_by_difficulty`
- Boss-gated unlock progression:
  - `squad::tests::upgrade_tier_unlocks_follow_major_boss_defeats`
  - `squad::tests::hero_tier_unlock_state_stays_locked_before_wave_60_major_defeat`
- Hero recruit and token economy:
  - `squad::tests::hero_recruit_requires_unlock_and_consumes_token_on_success`
  - `squad::tests::hero_recruit_requires_gold_and_does_not_consume_token_on_failure`
  - `upgrades::tests::hero_recruit_consumes_exactly_one_hear_the_call_token`
- Faction-aware items/icons and deterministic drops:
  - `inventory::tests::roll_chest_items_with_catalog_respects_faction_filters`
  - `inventory::tests::tier_and_hero_equipment_stays_scoped_to_matching_setup`
  - `drops::tests::equipment_drop_chance_scales_by_lane_and_wave`
  - `drops::tests::hear_the_call_drop_chance_scales_by_lane_wave_and_stash`

## Manual QA Sweep Matrix
| Faction | Difficulty | Focus | Pass Condition |
| --- | --- | --- | --- |
| Christian | Recruit | tier unlock pacing | Tier unlocks only after major-army kills |
| Christian | Experienced | strategy variance | Army composition differs by strategy roll |
| Christian | Alone Against the Infidels | anti-entry pressure | Formation tradeoffs remain readable/winnable |
| Muslim | Recruit | hero flow | Token + gold hero recruit behavior matches rules |
| Muslim | Experienced | item progression | Gear quality and drops ramp without runaway |
| Muslim | Alone Against the Infidels | late-wave stability | Waves 60-98 remain stable and deterministic |

## Pass/Fail Gates
- Full quality loop is green.
- No wave overflow into next wave while enemies are alive.
- Tier unlocks are never granted by wave index alone.
- Hero recruitment requires both:
  - `Hear the Call >= 1`
  - sufficient gold
- No blocker regressions in faction-aware item drop/icon paths.

## Residual Risk
- Balance drift can still occur when adding new doctrine cards or unit branches.
- Mitigation: keep this matrix and deterministic tests mandatory for future expansion patches.
