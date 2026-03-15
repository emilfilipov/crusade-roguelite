# AGENTS.md

## Project Summary
This project is a small 2D survivor-like game inspired by the era of Baldwin IV and Saladin, especially the Battle of Montgisard. The core twist is that the player does not control a single hero. Instead, the player controls the movement and positioning of an entire squad.

The squad auto-attacks, and the player grows stronger by recruiting troops, improving equipment, changing formations, and maintaining morale. The game should feel like a mix of survivor combat and light tactical army management, while staying small in scope.

The tone should be grounded, dusty, historical-inspired, and readable rather than strictly realistic.

## Technical Direction
- Engine and language: Rust + Bevy only.
- Architecture: ECS-first, modular Bevy plugins, data-driven gameplay values.
- Primary build target: Windows (`x86_64-pc-windows-msvc`).
- Distribution target: Steam (when account/app setup is available).
- Local testing/distribution requirement: produce a Windows installer for non-Steam local testing.
- Steam integration must be optional/feature-gated so local builds run without Steam runtime dependencies.

## Tooling and Data Conventions
- Rust toolchain must be stable and pinned via `rust-toolchain.toml`.
- Keep gameplay tuning in data files under `assets/data` (RON or JSON), not hardcoded constants where avoidable.
- Keep placeholder-friendly art/audio pipelines so gameplay iteration is not blocked by content polish.

## Scope Gating and Expansion Registry
- The source of truth for "limited now vs expandable later" is `SYSTEM_SCOPE_MAP.md`.
- Default to one-example-first implementation per gameplay system in MVP.
- Do not add a second variant/type/subsystem until the first variant is stable and tested.
- When expanding a system beyond its MVP limit, update `SYSTEM_SCOPE_MAP.md` and `TASKS.md` in the same change.

## Core Design Pillars
1. The squad is the player character and moves as a formation.
2. Start with one formation only (`Square`) and expand later.
3. Each squad member is an individual soldier with their own stats, weapon, armor, skills, and level progression.
4. Each run starts with only the commander (`Baldiun`), focused on support (auras, formation control, battle cries) but still capable of basic auto-attacks.
5. Squad growth comes from rescuing soldiers in the level; rescued soldiers join the retinue.
6. Retinue size is unlimited in design intent (practically constrained only by performance/balance during implementation).
7. Formation and positioning matter more than twitch combat.
8. Upgrades should feel like building an army, not leveling a lone hero.
9. Morale, cohesion, and banner survival are central to the game feel.
10. Keep scope small and prototype-friendly.

## MVP Scope
Build the smallest playable version first.

### Player-controlled squad
- One controllable squad blob that starts with only the commander
- Auto-attacking commander and squad units
- Simple movement only
- Square formation only in early prototype

### Starting unit types
- One recruitable unit archetype for initial scaffold:
  - Infantry (first implemented subtype: Knight)
- Additional archetypes/subtypes are post-scaffold additions

### Starting formations
- Square only: better defense / anti-cavalry
- Line and others are deferred until core loop is stable

### Starting enemy types
- One implemented enemy archetype for initial scaffold:
  - Infantry (melee)
- Archers and cavalry are deferred until core combat loop is stable

### First map
- One desert battlefield
- Minimal terrain
- Optional oasis healing zone

### Upgrade categories
- Add units
- Improve armor
- Improve damage
- Improve attack speed
- Improve morale/cohesion

### Recruitment loop
- Neutral/rescuable soldiers appear in the level
- Player rescues them by standing near them for a fixed channel duration
- On successful rescue, they join the retinue immediately
- No hard retinue cap for MVP unless performance requires a temporary cap

## Important Mechanics
### Squad health model
Prefer a combination of:
- Individual soldiers can die
- The squad also has morale/cohesion

Low cohesion should reduce effectiveness, such as:
- slower attacks
- looser formation
- weaker defense
- risk of collapse

### Banner system
The banner is the squad's anchor.
- If the banner falls, morale drops
- Formation discipline weakens
- The player may recover it by moving to it

### Commander role
Baldiun should not be treated as a solo action hero. He should function more like a fragile command center that supports the army through leadership, morale, and control radius.
He starts each run alone and has a basic auto-attack so early progression is always possible before first rescue.

## Desired Gameplay Feel
The run should create decisions like:
- elite small force vs larger weaker force
- defensive formation vs aggressive charge
- ranged composition vs melee composition
- protecting support units vs pushing forward

## Style Guidance
- Top-down or slightly angled 2D presentation
- Clear silhouettes and readable battlefield state
- Dusty desert palette
- Minimal but atmospheric historical flavor
- Avoid excessive realism if it hurts gameplay clarity

## Constraints
- Do not expand into a full RTS
- Do not add large-scale world simulation
- Do not overbuild historical systems
- Prefer simple, readable mechanics over realism
- Prototype first, deepen later

## Instructions for Coding Agents
When contributing to this project, follow these rules:

1. Preserve the core concept: the player controls a squad, not a lone hero.
2. Favor simple systems that support fast prototyping.
3. Keep units, upgrades, and formations data-driven where possible.
4. Maintain clear separation between:
   - unit logic
   - squad formation logic
   - enemy spawning
   - upgrade selection
   - morale/cohesion systems
5. Do not introduce unnecessary engine complexity.
6. Prefer readable, modular code over clever abstractions.
7. Add new mechanics only if they support the main loop of squad survival and growth.
8. Keep visuals placeholder-friendly so gameplay can be tested early.
9. When unsure, choose the smaller implementation.
10. Any new feature should be justifiable within MVP scope or clearly marked as post-MVP.
11. Keep code organized by domain modules/plugins (for example: squad, formation, combat, morale, enemies, upgrades, ui, packaging hooks).
12. Prefer deterministic, testable pure logic for combat/math systems; isolate Bevy-specific glue from core logic.

## Build, Test, and Repository Discipline
These steps are mandatory for every meaningful code change.

1. Run formatting and static checks:
   - `cargo fmt --all`
   - `cargo clippy --all-targets --all-features -- -D warnings`
2. Run automated tests:
   - `cargo test --all-targets --all-features`
3. Build a release binary for the Windows target:
   - `cargo build --release --target x86_64-pc-windows-msvc`
4. If any build/check/test step fails, investigate, fix, and rerun the full loop until all steps succeed.
5. Do not stop on red tests or failed builds. Resolve issues before moving on.
6. All logic that can be hardened by unit tests must be covered by unit tests.
7. Unit tests must pass at all times in the repository state being delivered.
8. After a successful build/test cycle, push all changes to the repository.
9. For local distribution testing, ensure the Windows installer build step also passes (for example via Inno Setup script).
10. Documentation changes (including all `.md` files) must also be committed and pushed; do not leave markdown-only changes hanging locally.

## Release Packaging Requirement
- Maintain an installer script and packaging flow for Windows local QA builds.
- Expected output artifact for local testers: installable `.exe` installer containing game executable and required assets.
- Steam packaging can be added in parallel, but local installer support is required from early development onward.

## Priorities for the First Prototype
1. Squad movement with square formation
2. Commander-only start and commander auto-combat
3. Rescue-to-recruit loop
4. Squad auto-combat
5. Enemy waves
6. Unit death
7. Upgrade selection between level-ups
8. Morale/cohesion
9. Banner failure/recovery

## Post-MVP Ideas
Only after the core loop works:
- Sergeant or priest support units
- Additional formations like line, wedge, or column
- Terrain points of interest
- Battle events instead of traditional bosses
- Light meta-progression
- More faction flavor
