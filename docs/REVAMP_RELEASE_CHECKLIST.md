# REVAMP_RELEASE_CHECKLIST.md

## Scope
Release integration checklist for deterministic progression revamp (`CRU-212`).

## Migration Notes (Final)
- Legacy upgrade roll fields removed:
  - `min_value`, `max_value`, `value_step`, `weight_exponent`
- Legacy upgrade IDs blocked with replacement hints:
  - `fast_learner_up*` -> `quartermaster_up`
  - deprecated mob variants -> consolidated IDs
- Draft lane semantics are canonical:
  - `Minor`: non-milestone levels
  - `Major`: levels divisible by `5`
- Itemization is deterministic:
  - template-driven values, authored `scrap_gold_value`, no runtime stat-roll drift.

## Packaging Smoke Contract
- `cargo build --release --target x86_64-pc-windows-msvc` succeeds.
- Data boot smoke succeeds via tests loading `assets/data` with strict validators.
- No runtime references remain to removed roll-tier mechanics in docs or validation.

## Release Sign-off Checklist
- [x] Revamp QA matrix present (`docs/REVAMP_QA_MATRIX.md`)
- [x] Expansion QA matrix present (`docs/EXPANSION_QA_MATRIX.md`)
- [x] Deterministic draft replay tests included
- [x] Doctrine upside/downside regressions covered
- [x] Gold/token/item deterministic economy checks covered
- [x] Final task board statuses synchronized in `docs/TASKS.md`
