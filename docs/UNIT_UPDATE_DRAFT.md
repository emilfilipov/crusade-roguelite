# UNIT_UPDATE_DRAFT.md

## Purpose
This draft defines a faction-agnostic unit update pass: names, role identity, qualitative stat profile, core ability, strengths, and weaknesses.

This is a design/update draft, not a hard schema migration file.

## Stat Band Key
- `|....` Very Low
- `||...` Low
- `|||..` Moderate
- `||||.` High
- `|||||` Very High

## Profile Columns
- `Durability`: health/survivability profile.
- `Armor`: mitigation profile.
- `Damage`: direct DPS profile.
- `Tempo`: attack cadence (higher is faster).
- `Range`: attack reach.
- `Speed`: movement profile.

## Infantry Tree (Tier 0 -> Tier 5)
| Unit | Role | Profile (Dur/Arm/Dmg/Tempo/Range/Speed) | Core Ability | Strengths | Weaknesses |
|---|---|---|---|---|---|
| Peasant Infantry (T0) | Frontline starter | `|||.. / ||... / ||... / |||.. / ||... / |||..` | Basic melee auto-attack | Cheap frontline body, Shielded baseline | Low kill pressure |
| Men-at-Arms (T1) | Frontline | `||||. / |||.. / ||... / ||... / ||... / ||...` | Basic melee auto-attack | Better hold line than T0 | Slower tempo, still low burst |
| Shield Infantry (T2) | Tank | `||||. / ||||. / ||... / ||... / ||... / ||...` | Basic melee auto-attack | Durable anchor, Shielded + Block-capable | Weak chase and finishing |
| Experienced Shield Infantry (T3) | Tank | `||||. / ||||. / ||... / ||... / ||... / ||...` | Basic melee auto-attack | Stable anti-collapse frontline | Low offensive value |
| Elite Shield Infantry (T4) | Tank | `||||| / ||||. / ||... / ||... / ||... / ||...` | Basic melee auto-attack | Very high mitigation uptime | Can be kited by high mobility |
| Citadel Guard (T5) | Tank capstone | `||||| / ||||| / |||.. / ||... / ||... / ||...` | Basic melee auto-attack | Best pure hold-line unit, Hero doctrine base | Low map pressure without support |
| Spearman (T2) | Anti-cavalry | `|||.. / |||.. / |||.. / |||.. / |||.. / ||...` | Extended-reach melee | Counters cavalry dives | Lower pressure into support backlines |
| Shielded Spearman (T3) | Anti-cavalry | `||||. / ||||. / |||.. / |||.. / |||.. / ||...` | Extended-reach melee | Anti-cavalry + Shielded resilience | Less burst than bruiser line |
| Halberdier (T4) | Anti-cavalry | `||||. / ||||. / ||||. / ||... / |||.. / ||...` | Extended-reach melee | Strong cavalry denial, high armor pressure | Slower into spread skirmish targets |
| Armored Halberdier (T5) | Anti-cavalry capstone | `||||| / ||||| / ||||. / ||... / |||.. / ||...` | Extended-reach melee | Elite anti-cavalry, anti-armor hook, Hero doctrine base | Expensive and tempo-limited |
| Unmounted Knight (T2) | Bruiser | `|||.. / ||||. / ||||. / |||.. / ||... / |||..` | Aggressive melee auto-attack | High frontline kill pressure | Lower sustain than tank branch |
| Knight (T3) | Bruiser | `||||. / ||||. / ||||. / |||.. / ||... / |||..` | Aggressive melee auto-attack | Balanced offense/defense | Can be out-traded by dedicated tanks |
| Heavy Knight (T4) | Bruiser | `||||. / ||||| / ||||. / ||... / ||... / |||..` | Aggressive melee auto-attack | High pressure with strong armor | Slower tempo than cavalry branch |
| Elite Heavy Knight (T5) | Bruiser capstone | `||||| / ||||| / ||||| / ||... / ||... / |||..` | Aggressive melee auto-attack | Top melee duelist pressure, Hero doctrine base | Vulnerable to anti-cavalry clusters when overextended |
| Squire (T2) | Support | `|||.. / ||... / |.... / |||.. / |.... / |||..` | Blessing pulse support | Stabilizes formation uptime | Very low direct damage |
| Bannerman (T3) | Support | `||||. / |||.. / |.... / |||.. / |.... / |||..` | Blessing pulse support | Better sustain/utility uptime | Weak in isolated duels |
| Elite Bannerman (T4) | Support | `||||. / ||||. / |.... / |||.. / |.... / |||..` | Blessing pulse support | Strong aura uptime and survivability | Limited kill contribution |
| God's Chosen (T5) | Support capstone | `||||| / ||||. / |.... / |||.. / |.... / |||..` | Blessing pulse support | Best support frontline glue | Needs damage branches around it |

## Ranged/Skirmisher Tree (Tier 0 -> Tier 5)
| Unit | Role | Profile (Dur/Arm/Dmg/Tempo/Range/Speed) | Core Ability | Strengths | Weaknesses |
|---|---|---|---|---|---|
| Peasant Archer (T0) | Hybrid ranged | `||... / ||... / |||.. / ||... / ||||. / |||..` | Auto-ranged + melee fallback | Early ranged value | Fragile under dives |
| Bowman (T1) | Ranged | `||... / |.... / |||.. / ||||. / ||||. / |||..` | Auto-ranged + melee fallback | Stable sustained volleys | Low armor |
| Experienced Bowman (T2) | Ranged | `||... / |.... / |||.. / ||||. / ||||. / ||||.` | Auto-ranged + melee fallback | Better sustained pressure | Punishable if screened poorly |
| Elite Bowman (T3) | Ranged | `||... / ||... / ||||. / ||||. / ||||. / ||||.` | Auto-ranged + melee fallback | Strong sustained ranged DPS | Weak against direct cavalry contact |
| Longbowman (T4) | Ranged | `||... / ||... / ||||. / ||||. / ||||| / ||||.` | Auto-ranged + melee fallback | Longest sustained reach | Collapses if line breaks |
| Elite Longbowman (T5) | Ranged capstone | `|||.. / ||... / ||||| / ||||. / ||||| / ||||.` | Auto-ranged + melee fallback | Peak sustained ranged line, Hero doctrine base | Requires protection investment |
| Crossbowman (T2) | Heavy ranged | `||... / ||... / ||||. / ||... / |||.. / |||..` | Heavy bolt shots + melee fallback | Anti-armor role starts here | Slower cadence |
| Armored Crossbowman (T3) | Heavy ranged | `|||.. / |||.. / ||||. / ||... / |||.. / |||..` | Heavy bolt shots + melee fallback | Better durability than bow line | Lower swarm clear tempo |
| Elite Crossbowman (T4) | Heavy ranged | `|||.. / ||||. / ||||| / ||... / |||.. / |||..` | Heavy bolt shots + melee fallback | High anti-armor pressure | Less efficient vs unarmored swarms |
| Siege Crossbowman (T5) | Heavy ranged capstone | `||||. / ||||. / ||||| / ||... / |||.. / |||..` | Heavy bolt shots + melee fallback | Best anti-armor ranged unit, Hero doctrine base | Slow tempo, needs spacing |
| Tracker (T2) | Skirmish ranged | `||... / ||... / |||.. / ||||. / ||||. / ||||.` | Periodic hound strike summons | Backline disruption, map pressure | Fragile in long melee trades |
| Pathfinder (T3) | Skirmish ranged | `||... / ||... / |||.. / ||||. / ||||. / ||||.` | Periodic hound strike summons | Better disruption uptime | Lower frontline impact |
| Houndmaster (T4) | Skirmish ranged | `|||.. / ||... / ||||. / ||||. / ||||. / |||||` | Periodic hound strike summons | High mobility disruption play | Needs careful target priority |
| Elite Houndmaster (T5) | Skirmish capstone | `|||.. / |||.. / ||||. / ||||. / ||||. / |||||` | Periodic hound strike summons | Elite support-hunting pressure, Hero doctrine base | Lower direct siege DPS than crossbows |
| Scout (T2) | Raider | `||... / ||... / |||.. / |||.. / ||... / |||||` | Periodic autonomous raid movement | Dive and backline pick potential | Vulnerable to anti-cavalry |
| Mounted Scout (T3) | Raider | `|||.. / ||... / |||.. / |||.. / ||... / |||||` | Periodic autonomous raid movement | Better raid uptime and returns | Low armor in prolonged brawls |
| Shock Cavalry (T4) | Raider | `|||.. / |||.. / ||||. / |||.. / ||... / |||||` | Periodic autonomous raid movement | Strong charge pressure vs frontline | Hard-countered by anti-cavalry |
| Elite Shock Cavalry (T5) | Raider capstone | `||||. / ||||. / ||||| / |||.. / ||... / |||||` | Periodic autonomous raid movement | Peak engage tempo, Hero doctrine base | Positioning-sensitive into spear walls |

## Support/Zealot Tree (Tier 0 -> Tier 5)
| Unit | Role | Profile (Dur/Arm/Dmg/Tempo/Range/Speed) | Core Ability | Strengths | Weaknesses |
|---|---|---|---|---|---|
| Peasant Priest (T0) | Support starter | `|||.. / ||... / |.... / ||... / |.... / |||..` | Blessing pulse support | Early sustain utility | Very low direct combat output |
| Devoted (T1) | Support | `|||.. / ||... / |.... / ||... / |.... / |||..` | Blessing pulse support | Better support uptime | Vulnerable when isolated |
| Devoted One (T2) | Support | `||||. / ||... / |.... / ||... / |.... / |||..` | Blessing pulse support | Durable support anchor | Low finishing power |
| Cardinal (T3) | Support | `||||. / |||.. / |.... / ||... / |.... / |||..` | Blessing pulse support | Strong aura stability | Struggles vs direct dive comps |
| Elite Cardinal (T4) | Support | `||||. / |||.. / |.... / ||... / |.... / |||..` | Blessing pulse support | High utility uptime | Needs frontline/bodyguard support |
| Divine Speaker (T5) | Support capstone | `||||| / |||.. / |.... / ||... / |.... / |||..` | Blessing pulse support | Best sustain/morale support, Hero doctrine base | Minimal direct DPS |
| Fanatic (T2) | Zealot | `|||.. / |.... / ||||. / ||||. / ||... / ||||.` | Life-leech melee (Armor locked to zero) | High melee pressure with sustain | Fragile under burst focus |
| Flagellant (T3) | Zealot | `|||.. / |.... / ||||. / ||||. / ||... / ||||.` | Life-leech melee (Armor locked to zero) | Better skirmish lethality | Still vulnerable to ranged focus |
| Elite Flagellant (T4) | Zealot | `||||. / |.... / ||||| / ||||. / ||... / ||||.` | Life-leech melee (Armor locked to zero) | Strong attrition finisher | No armor scaling |
| Divine Judge (T5) | Zealot capstone | `||||. / |.... / ||||| / ||||. / ||... / ||||.` | Life-leech melee (Armor locked to zero) | Peak zealot kill pressure, Hero doctrine base | Countered by focus fire and control |

## Hero Subtype Update
Hero recruitment remains subtype-driven; each subtype rolls a faction-specific name entry and uses the mapped tier-5 base unit profile.

| Hero Subtype | Base Unit Profile | Core Strengths | Core Weaknesses | Matchup Identity |
|---|---|---|---|---|
| Sword and Shield | Citadel Guard | Shield anchor, high durability | Low chase pressure | Holds frontline, weak at skirmish pursuit |
| Spear | Armored Halberdier | Anti-cavalry specialist, reach control | Lower support pressure | Wins into cavalry-heavy armies |
| Two-Handed Sword | Elite Heavy Knight | Burst melee breaker | Higher incoming damage | Frontline breaker, weak if focus-fired |
| Bow | Elite Longbowman | Sustained ranged tempo | Vulnerable to cavalry dives | Strong vs skirmishers/frontline attrition |
| Javelin | Siege Crossbowman | Premium anti-armor | Lower value vs unarmored swarms | Best into armored/heavy targets |
| Beast Master | Elite Houndmaster | Backline/support disruption | Reduced frontline DPS | Hunts support clusters |
| Super Priest | Divine Speaker | Sustain specialist, durable support | Low direct damage | Wins long fights, weak solo pressure |
| Super Fanatic | Divine Judge | Aggressive sustain and melee threat | Fragile under focus | Frontline attrition specialist |
| Super Knight | Elite Shock Cavalry | Charge tempo and engage pressure | Hard countered by anti-cavalry | Punishes frontline drift and exposed backlines |

## Naming Direction (Follows Current Refactor)
- Keep `unit_id` generic and faction-agnostic.
- Keep faction expression in override data: display name, icon, VFX/audio flavor, optional stat/ability deltas.
- Keep hero identity split:
  - subtype defines role and gameplay identity,
  - faction pool defines names and cosmetic flavor.

## Balance Notes for Next Pass
- Preserve clear branch identities: `Tank`, `Anti-Cavalry`, `Bruiser`, `Support`, `Ranged`, `Anti-Armor`, `Skirmisher`, `Raider`, `Zealot`.
- Keep `Shielded` limited to intended block-capable lines.
- Keep `AntiArmor` penalties visible against unarmored swarms.
- Keep cavalry pressure high, but always counterable through anti-cavalry and formation choices.
