# REVAMP_QA_MATRIX.md

## Purpose
Release-readiness matrix for the strategic progression revamp (`CRU-210`, `CRU-211`, `CRU-262`).
This file captures automated coverage anchors, manual validation runs, and pass/fail gates.

## Automated Coverage Anchors
- Upgrade cadence and level target:
  - `upgrades::tests::wave_98_grants_double_level_reward`
  - `upgrades::tests::wave_98_reward_reaches_level_100_by_end_of_wave_98`
  - `upgrades::tests::reward_kind_switches_to_major_on_every_fifth_level`
  - `upgrades::tests::major_minor_counts_follow_shared_level_parity_formula`
- Draft lane reproducibility and deterministic rolls:
  - `upgrades::tests::draft_rolls_are_reproducible_for_identical_seed_and_inputs`
  - `upgrades::tests::deterministic_roll_always_returns_authored_value`
  - `upgrades::tests::reward_lane_routing_keeps_major_and_minor_pools_separate`
- Doctrine upside/downside behavior:
  - `upgrades::tests::command_net_doctrine_applies_upside_and_downside`
  - `upgrades::tests::countervolley_doctrine_applies_conditional_upside_and_downside`
  - `upgrades::tests::pike_hedgehog_doctrine_applies_conditional_upside_and_downside`
- Economy sanity and opportunity cost:
  - `squad::tests::economy_affordability_profile_preserves_opportunity_costs`
  - `squad::tests::tier0_conversion_cost_has_fixed_gold_price`
  - `inventory::tests::scrap_value_is_deterministic_and_authored_per_template`

## Manual QA Matrix
Run at least one full sweep per row.

| Faction | Difficulty | Commander | Doctrine Focus | Expected Result |
| --- | --- | --- | --- | --- |
| Christian | Recruit | baseline_balanced | sustain/control | Stable clear to wave 60+, no economy deadlock |
| Christian | Experienced | baseline_aggressive | tempo/execute | Faster pacing, higher pressure, still recoverable |
| Christian | Alone Against the Infidels | baseline_defender | anti-cavalry/formation | Shielded lines and anti-cavalry picks matter |
| Muslim | Recruit | baseline_balanced | sustain/control | Stable clear to wave 60+, no economy deadlock |
| Muslim | Experienced | baseline_aggressive | tempo/execute | Commander choice shifts viable picks materially |
| Muslim | Alone Against the Infidels | baseline_defender | anti-ranged/formation | High-pressure lanes remain winnable with tradeoffs |

## Pass/Fail Thresholds
- `cargo test --all-targets --all-features` must pass with zero failures.
- No deterministic replay drift for draft/economy tests under fixed seeds.
- At least two distinct doctrine families must reach wave 60+ in manual sweeps per faction.
- No single doctrine/faction pair is allowed to exceed all others by >20% win-rate in internal runs.
- No blocker-tier issues in:
  - level cadence (`1..98`),
  - boss-gated tier unlocks,
  - hero token spend and recruit flow,
  - deterministic item/drop behavior.

## Certification Notes
- Revamp coverage now includes cadence, lane routing, deterministic draft replay, doctrine tradeoffs, and economy opportunity-cost checks.
- Remaining risk is balance drift from future content additions; mitigated by keeping deterministic replay tests and this matrix mandatory for new doctrine/item changes.
