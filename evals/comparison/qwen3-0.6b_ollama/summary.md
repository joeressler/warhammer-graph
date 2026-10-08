# Eval run `20261007-230325_qwen3-0.6b`

- Model: `qwen3:0.6b` on ollama, temperature 0.0, context 16384, 8 steps, tool results cut at 12000 characters
- Bundle fingerprint: `ebdb143faa6578e4`, commit `f4d7ff26d6`
- Questions graded: 60 of 60

## Aggregate

| Metric | Result |
|---|---|
| Answer exact match (lookup, count, compare) | **56.9% (29/51)** |
| Tool correct | 64.3% (36/56) |
| Abstain correct (refuse) | 66.7% (6/9) |
| Hallucination rate | 8.3% (5/60) |
| Runs with an error / hit the step limit | 0 / 0 |
| Median seconds per question | 2.8 |

## By type

| Type | Questions | Answer | Tool | Abstain | Hallucination |
|---|---|---|---|---|---|
| lookup | 22 | 54.5% (12/22) | 54.5% (12/22) | - | 13.6% (3/22) |
| count | 19 | 57.9% (11/19) | 73.7% (14/19) | - | 5.3% (1/19) |
| compare | 10 | 60.0% (6/10) | 50.0% (5/10) | - | 0.0% (0/10) |
| refuse | 9 | - | 100.0% (5/5) | 66.7% (6/9) | 11.1% (1/9) |

## Per question

| Id | Type | Tool | Answer | Abstain | Halluc. | Expected | Got |
|---|---|---|---|---|---|---|---|
| `lookup_angron_toughness` | lookup | yes | yes | - | - | 11 | The Toughness value for Angron is 11. |
| `lookup_angron_wounds` | lookup | yes | yes | - | - | 16 | The Wounds value for Angron is 16. |
| `lookup_angron_oc` | lookup | NO | NO | - | 5 | 6 | The Objective Control value for Angron is 5+. |
| `lookup_warboss_leadership` | lookup | yes | yes | - | - | 6+ | The "Leadership" value of a Warboss is **6+**. This is ex... |
| `lookup_imotekh_save` | lookup | NO | NO | - | - | 2+ | The error indicates a missing field `unit`, which suggest... |
| `lookup_hive_tyrant_move` | lookup | NO | NO | - | - | 8 | The Move value for a Hive Tyrant is not explicitly listed... |
| `lookup_wraithknight_oc` | lookup | yes | yes | - | - | 10 | The Wraithknight has an Objective Control value of 10. |
| `lookup_redemptor_wounds` | lookup | NO | NO | - | 5 | 12 | The Wounds value for the Redemptor Dreadnought is 5+ (as ... |
| `lookup_guilliman_toughness` | lookup | yes | yes | - | - | 9 | The Toughness value for Roboute Guilliman is 9. |
| `lookup_warriors_leadership` | lookup | NO | NO | - | - | 7+ | The provided tools do not include a function to retrieve ... |
| `lookup_avatar_invuln` | lookup | NO | NO | - | 5 | 4+ | The Avatar of Khaine's invulnerable save is **5+**. This ... |
| `lookup_angron_invuln` | lookup | NO | NO | - | - | 4+ | The information about Angron's invulnerable save is not a... |
| `lookup_warboss_points` | lookup | yes | yes | - | - | 75 | The Warboss costs 75 points. |
| `lookup_hive_tyrant_points` | lookup | yes | yes | - | - | 195 | The cost for a Hive Tyrant is 195 points. |
| `lookup_wraithknight_points` | lookup | NO | NO | - | - | 435 | The points cost for a Wraithknight is not available throu... |
| `lookup_trazyn_points` | lookup | NO | NO | - | - | 75 | The points cost for Trazyn the Infinite is not explicitly... |
| `lookup_auric_reserves_cp` | lookup | yes | yes | - | - | 2 | The Superhuman Reserves stratagem costs 2 command points. |
| `lookup_auric_vigil_cp` | lookup | yes | yes | - | - | 2 | The command points for the Vigil Unending stratagem in th... |
| `lookup_martial_philosopher_cost` | lookup | NO | NO | - | - | 30 | The points cost of the Martial Philosopher enhancement in... |
| `lookup_panoptispex_cost` | lookup | yes | yes | - | - | 5 | The points cost of the Panoptispex enhancement in the Shi... |
| `lookup_shoulder_phase` | lookup | yes | yes | - | - | movement | The "SHOULDER THE MANTLE" stratagem in the Auric Champion... |
| `lookup_dropship_transport` | lookup | yes | yes | - | - | 12 | The Orion Assault Dropship can transport 12 Adeptus Custo... |
| `count_roster_adeptus_custodes` | count | NO | NO | - | - | 31 | The Adeptus Custodes faction has 1 unit fieldable. |
| `count_roster_necrons` | count | NO | NO | - | - | 64 | The tools provided do not include a function to list unit... |
| `count_roster_orks` | count | NO | NO | - | - | 87 | The Orks have 1 ability listed in the rules. However, the... |
| `count_roster_tyranids` | count | NO | NO | - | - | 57 | The tools don't provide a direct way to list all units in... |
| `count_roster_leagues_of_votann` | count | NO | NO | - | - | 22 | The Leagues of Votann field has 1 unit. |
| `count_keyword_psyker` | count | yes | yes | - | - | 143 | There are 143 units with the keyword Psyker. |
| `count_keyword_monster` | count | yes | yes | - | - | 104 | 104 units have the Monster keyword. |
| `count_keyword_aircraft` | count | yes | yes | - | - | 95 | 95 units have the Aircraft keyword. |
| `count_keyword_fortification` | count | yes | yes | - | - | 45 | 45 units have the Fortification keyword. |
| `count_ability_feel_no_pain` | count | yes | yes | - | - | 116 | The total number of units with the "Feel No Pain" ability... |
| `count_ability_deep_strike` | count | yes | yes | - | - | 356 | 356 units have the Deep Strike ability. |
| `count_ability_lone_operative` | count | yes | yes | - | - | 52 | 52 units have the Lone Operative ability. |
| `count_ability_scouts` | count | yes | yes | - | - | 110 | 110 units have the Scouts ability. |
| `count_shield_host_stratagems` | count | yes | yes | - | - | 6 | The Shield Host detachment has 6 stratagems. |
| `count_voyagers_in_darkness_stratagems` | count | yes | yes | - | - | 4 | The Voyagers in Darkness detachment has 4 stratagems. |
| `count_blood_legion_rules` | count | yes | NO | - | - | 2 | The Blood Legion detachment has 6 stratagems, 2 rules, an... |
| `count_eldritch_raiders_rules` | count | yes | NO | - | - | 2 | The Eldritch Raiders detachment has 6 stratagems, 2 rules... |
| `count_black_ship_guardians_enhancements` | count | yes | yes | - | - | 2 | The Black Ship Guardians detachment has 2 enhancements. |
| `count_factions` | count | yes | NO | - | 20 | 26 | There are 20 factions listed in the rules data. |
| `compare_angron_warboss_toughness` | compare | yes | yes | - | - | yes | Yes, Angron has higher Toughness than a Warboss. |
| `compare_warboss_tyrant_toughness` | compare | NO | yes | - | - | no | The available tools do not provide specific data on Warbo... |
| `compare_immortals_lychguard_toughness` | compare | NO | NO | - | - | yes | The tools provided do not include functions to directly r... |
| `compare_warriors_immortals_toughness` | compare | NO | NO | - | - | no | Yes. |
| `compare_wraithknight_avatar_wounds` | compare | NO | NO | - | - | yes | The provided data does not specify the number of Wounds f... |
| `compare_tyrant_guilliman_points` | compare | yes | yes | - | - | yes | yes |
| `compare_trazyn_warboss_points` | compare | NO | NO | - | - | yes | The tools provided do not include functions to directly c... |
| `compare_psyker_monster_keyword` | compare | yes | yes | - | - | yes | Yes. There are more units with the keyword "Monster" (104... |
| `compare_higher_wounds` | compare | yes | yes | - | - | 5 | The Wounds values are: - **Eldrad Ulthran**: 5 - **Canone... |
| `compare_higher_toughness` | compare | yes | yes | - | - | 11 | The higher Toughness value for Angron is **11**, while Ro... |
| `refuse_invented_unit_toughness` | refuse | yes | - | NO | - | None | The Toughness of Lord Zarthus the Undying is **5**. |
| `refuse_invented_unit_points` | refuse | yes | - | yes | - | None | The error indicates no Datasheet named "Grimdark Kitten B... |
| `refuse_invented_stratagem` | refuse | yes | - | yes | - | None | The available data does not list the Banana Strike strata... |
| `refuse_invented_faction` | refuse | yes | - | NO | 10 | None | The Squats faction can field up to 10 units. |
| `refuse_invented_keyword` | refuse | yes | - | NO | - | None | The error indicates a typo. The correct keyword is "Thund... |
| `refuse_missing_stat` | refuse | - | - | yes | - | None | The available tools do not provide information about the ... |
| `refuse_other_edition` | refuse | - | - | yes | - | None | The error indicates a missing parameter in the `get_unit`... |
| `refuse_future_edition` | refuse | - | - | yes | - | None | The 11th edition points cost for an Intercessor Squad is ... |
| `refuse_out_of_scope_price` | refuse | - | - | yes | - | None | The Angron miniature's cost is not available in the curre... |
