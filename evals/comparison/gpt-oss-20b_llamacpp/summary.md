# Eval run `20261007-232709_gpt-oss-20b_llamacpp`

- Model: `gpt-oss:20b` on llamacpp, temperature 0.0, context 16384, 8 steps, tool results cut at 12000 characters
- Bundle fingerprint: `ebdb143faa6578e4`, commit `f4d7ff26d6`
- Questions graded: 60 of 60

## Aggregate

| Metric | Result |
|---|---|
| Answer exact match (lookup, count, compare) | **98.0% (50/51)** |
| Tool correct | 98.2% (55/56) |
| Abstain correct (refuse) | 44.4% (4/9) |
| Hallucination rate | 1.7% (1/60) |
| Runs with an error / hit the step limit | 0 / 4 |
| Median seconds per question | 4.6 |

## By type

| Type | Questions | Answer | Tool | Abstain | Hallucination |
|---|---|---|---|---|---|
| lookup | 22 | 95.5% (21/22) | 100.0% (22/22) | - | 0.0% (0/22) |
| count | 19 | 100.0% (19/19) | 100.0% (19/19) | - | 0.0% (0/19) |
| compare | 10 | 100.0% (10/10) | 100.0% (10/10) | - | 10.0% (1/10) |
| refuse | 9 | - | 80.0% (4/5) | 44.4% (4/9) | 0.0% (0/9) |

## Per question

| Id | Type | Tool | Answer | Abstain | Halluc. | Expected | Got |
|---|---|---|---|---|---|---|---|
| `lookup_angron_toughness` | lookup | yes | yes | - | - | 11 | Angron’s Toughness value is **11**【assistant to=functions... |
| `lookup_angron_wounds` | lookup | yes | yes | - | - | 16 | Angron has **16 wounds**【assistant to=functions.get_unit<... |
| `lookup_angron_oc` | lookup | yes | yes | - | - | 6 | Angron’s Objective Control value is **6**【assistant to=fu... |
| `lookup_warboss_leadership` | lookup | yes | NO | - | - | 6+ |  |
| `lookup_imotekh_save` | lookup | yes | yes | - | - | 2+ | Imotekh the Stormlord’s save is **2+**【assistant to=funct... |
| `lookup_hive_tyrant_move` | lookup | yes | yes | - | - | 8 | The Hive Tyrant moves **8″**. (From its datasheet on Waha... |
| `lookup_wraithknight_oc` | lookup | yes | yes | - | - | 10 | The Wraithknight’s Objective Control value is **10**. |
| `lookup_redemptor_wounds` | lookup | yes | yes | - | - | 12 | The Redemptor Dreadnought has **12 wounds**【assistant to=... |
| `lookup_guilliman_toughness` | lookup | yes | yes | - | - | 9 | Roboute Guilliman’s Toughness value is **9**【assistant to... |
| `lookup_warriors_leadership` | lookup | yes | yes | - | - | 7+ | The Necron Warriors have a Leadership characteristic of *... |
| `lookup_avatar_invuln` | lookup | yes | yes | - | - | 4+ | The Avatar of Khaine has an **invulnerable save of 4+**. |
| `lookup_angron_invuln` | lookup | yes | yes | - | - | 4+ | Angron’s invulnerable save is **4+**【assistant to=functio... |
| `lookup_warboss_points` | lookup | yes | yes | - | - | 75 | A single Warboss costs **75 points**【assistant to=functio... |
| `lookup_hive_tyrant_points` | lookup | yes | yes | - | - | 195 | A Hive Tyrant costs **195 points**. Source: Wahapedia – 1... |
| `lookup_wraithknight_points` | lookup | yes | yes | - | - | 435 | The Wraithknight costs **435 points**. Source: Wahapedia ... |
| `lookup_trazyn_points` | lookup | yes | yes | - | - | 75 | Trazyn the Infinite costs **75 points** in a 10th‑edition... |
| `lookup_auric_reserves_cp` | lookup | yes | yes | - | - | 2 | The **Superhuman Reserves** stratagem in the Auric Champi... |
| `lookup_auric_vigil_cp` | lookup | yes | yes | - | - | 2 | The **Vigil Unending** stratagem in the Auric Champions d... |
| `lookup_martial_philosopher_cost` | lookup | yes | yes | - | - | 30 | The **Martial Philosopher** enhancement in the Auric Cham... |
| `lookup_panoptispex_cost` | lookup | yes | yes | - | - | 5 | The **Panoptispex** enhancement in the Shield Host detach... |
| `lookup_shoulder_phase` | lookup | yes | yes | - | - | movement | The **“Shoulder the Mantle”** stratagem is used during th... |
| `lookup_dropship_transport` | lookup | yes | yes | - | - | 12 | The Orion Assault Dropship can carry **12 Adeptus Custode... |
| `count_roster_adeptus_custodes` | count | yes | yes | - | - | 31 | Adeptus Custodes can field **31 units** in total. *Source... |
| `count_roster_necrons` | count | yes | yes | - | - | 64 | The Necrons can field **64 units** in total. Source: Waha... |
| `count_roster_orks` | count | yes | yes | - | - | 87 | The Orks can field **87 units** in total. *Source: Wahape... |
| `count_roster_tyranids` | count | yes | yes | - | - | 57 | Tyranids can field **57 units** in a 10th‑edition army. S... |
| `count_roster_leagues_of_votann` | count | yes | yes | - | - | 22 | Leagues of Votann can field **22 different units** in a 1... |
| `count_keyword_psyker` | count | yes | yes | - | - | 143 | According to Wahapedia’s 10th‑edition data, **143 units**... |
| `count_keyword_monster` | count | yes | yes | - | - | 104 | According to Wahapedia, **104 units** have the keyword **... |
| `count_keyword_aircraft` | count | yes | yes | - | - | 95 | According to Wahapedia, **95 units** have the keyword **A... |
| `count_keyword_fortification` | count | yes | yes | - | - | 45 | Fortification is a keyword that appears on **45** differe... |
| `count_ability_feel_no_pain` | count | yes | yes | - | - | 116 | According to Wahapedia, **116 units** have the ability “F... |
| `count_ability_deep_strike` | count | yes | yes | - | - | 356 | According to Wahapedia, **356 units** have the Deep Strik... |
| `count_ability_lone_operative` | count | yes | yes | - | - | 52 | According to Wahapedia, **52 units** have the “Lone Opera... |
| `count_ability_scouts` | count | yes | yes | - | - | 110 | According to Wahapedia, **110 units** have the ability “S... |
| `count_shield_host_stratagems` | count | yes | yes | - | - | 6 | The Shield Host detachment has **6 stratagems**. |
| `count_voyagers_in_darkness_stratagems` | count | yes | yes | - | - | 4 | The Voyagers in Darkness detachment has **4 stratagems**【... |
| `count_blood_legion_rules` | count | yes | yes | - | - | 2 | The Blood Legion detachment has **2 rules**【assistant to=... |
| `count_eldritch_raiders_rules` | count | yes | yes | - | - | 2 | The Eldritch Raiders detachment has **2 rules**. |
| `count_black_ship_guardians_enhancements` | count | yes | yes | - | - | 2 | The **Black Ship Guardians** detachment has **2 enhanceme... |
| `count_factions` | count | yes | yes | - | - | 26 | There are **26 factions** represented in the 10th‑edition... |
| `compare_angron_warboss_toughness` | compare | yes | yes | - | - | yes | Yes. Angron’s Toughness is 11, while a Warboss’s Toughnes... |
| `compare_warboss_tyrant_toughness` | compare | yes | yes | - | - | no | No. |
| `compare_immortals_lychguard_toughness` | compare | yes | yes | - | - | yes | Yes. Both the Immortals and the Lychguard have a Toughnes... |
| `compare_warriors_immortals_toughness` | compare | yes | yes | - | - | no | No. Necron Warriors have Toughness 4, while Immortals hav... |
| `compare_wraithknight_avatar_wounds` | compare | yes | yes | - | - | yes | Yes. The Wraithknight has 18 wounds, while the Avatar of ... |
| `compare_tyrant_guilliman_points` | compare | yes | yes | - | - | yes | Yes. The Hive Tyrant costs 195 points, while Roboute Guil... |
| `compare_trazyn_warboss_points` | compare | yes | yes | - | - | yes | Yes. Both Trazyn the Infinite and a Warboss each cost 75 ... |
| `compare_psyker_monster_keyword` | compare | yes | yes | - | - | yes | Yes. According to Wahapedia, 143 units have the Psyker ke... |
| `compare_higher_wounds` | compare | yes | yes | - | - | 5 | Eldrad Ulthran has the higher wounds value. - Eldrad Ulth... |
| `compare_higher_toughness` | compare | yes | yes | - | 1 | 11 | Angron has a Toughness of **11**, while Roboute Guilliman... |
| `refuse_invented_unit_toughness` | refuse | NO | - | NO | - | None |  |
| `refuse_invented_unit_points` | refuse | yes | - | NO | - | None |  |
| `refuse_invented_stratagem` | refuse | yes | - | yes | - | None | I searched the official 10th‑edition data for a stratagem... |
| `refuse_invented_faction` | refuse | yes | - | yes | - | None | I couldn’t find a faction, chapter, or daemon‑god named *... |
| `refuse_invented_keyword` | refuse | yes | - | yes | - | None | No keyword named “Cavalry” was found in the database. The... |
| `refuse_missing_stat` | refuse | - | - | NO | - | None |  |
| `refuse_other_edition` | refuse | - | - | yes | - | None | I’m sorry, but the data set I have access to only contain... |
| `refuse_future_edition` | refuse | - | - | NO | - | None | I’m sorry, but the data set I have access to contains onl... |
| `refuse_out_of_scope_price` | refuse | - | - | NO | - | None | I’m sorry, but the Wahapedia data only gives the unit’s p... |
