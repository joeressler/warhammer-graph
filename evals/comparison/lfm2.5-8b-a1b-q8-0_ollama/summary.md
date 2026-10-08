# Eval run `20261007-230638_lfm2.5-8b-a1b-q8-0`

- Model: `lfm2.5:8b-a1b-q8_0` on ollama, temperature 0.0, context 16384, 8 steps, tool results cut at 12000 characters
- Bundle fingerprint: `ebdb143faa6578e4`, commit `f4d7ff26d6`
- Questions graded: 60 of 60

## Aggregate

| Metric | Result |
|---|---|
| Answer exact match (lookup, count, compare) | **94.1% (48/51)** |
| Tool correct | 94.6% (53/56) |
| Abstain correct (refuse) | 11.1% (1/9) |
| Hallucination rate | 1.7% (1/60) |
| Runs with an error / hit the step limit | 0 / 6 |
| Median seconds per question | 4.3 |

## By type

| Type | Questions | Answer | Tool | Abstain | Hallucination |
|---|---|---|---|---|---|
| lookup | 22 | 100.0% (22/22) | 95.5% (21/22) | - | 0.0% (0/22) |
| count | 19 | 94.7% (18/19) | 94.7% (18/19) | - | 5.3% (1/19) |
| compare | 10 | 80.0% (8/10) | 90.0% (9/10) | - | 0.0% (0/10) |
| refuse | 9 | - | 100.0% (5/5) | 11.1% (1/9) | 0.0% (0/9) |

## Per question

| Id | Type | Tool | Answer | Abstain | Halluc. | Expected | Got |
|---|---|---|---|---|---|---|---|
| `lookup_angron_toughness` | lookup | yes | yes | - | - | 11 | Angron's Toughness value is 11. |
| `lookup_angron_wounds` | lookup | yes | yes | - | - | 16 | Angron’s initial Wounds value is 16. |
| `lookup_angron_oc` | lookup | yes | yes | - | - | 6 | Angron's Objective Control value is 6. |
| `lookup_warboss_leadership` | lookup | yes | yes | - | - | 6+ | The Warboss’s Leadership value is 6+. |
| `lookup_imotekh_save` | lookup | yes | yes | - | - | 2+ | The Save value for Imotekh the Stormlord is 2+. |
| `lookup_hive_tyrant_move` | lookup | yes | yes | - | - | 8 | The Hive Tyrant’s Move value is 8 inches. |
| `lookup_wraithknight_oc` | lookup | yes | yes | - | - | 10 | The Wraithknight’s Objective Control value is **10** (as ... |
| `lookup_redemptor_wounds` | lookup | yes | yes | - | - | 12 | The Redemptor Dreadnought has a Wounds value of 12. More ... |
| `lookup_guilliman_toughness` | lookup | yes | yes | - | - | 9 | Roboute Guilliman’s Toughness is 9 according to his offic... |
| `lookup_warriors_leadership` | lookup | yes | yes | - | - | 7+ | The Necron Warriors have a Leadership value of 7+. |
| `lookup_avatar_invuln` | lookup | yes | yes | - | - | 4+ | The Avatar of Khaine has an invulnerable save of **4+** (... |
| `lookup_angron_invuln` | lookup | yes | yes | - | - | 4+ | Angron’s invulnerable save is 4+. |
| `lookup_warboss_points` | lookup | yes | yes | - | - | 75 | The Warboss model costs 75 points. |
| `lookup_hive_tyrant_points` | lookup | yes | yes | - | - | 195 | The Hive Tyrant is listed with a point cost of 195 points... |
| `lookup_wraithknight_points` | lookup | yes | yes | - | - | 435 | The Wraithknight costs 435 points. |
| `lookup_trazyn_points` | lookup | yes | yes | - | - | 75 | 75 points |
| `lookup_auric_reserves_cp` | lookup | yes | yes | - | - | 2 | The Superhuman Reserves stratagem in the Auric Champions ... |
| `lookup_auric_vigil_cp` | lookup | yes | yes | - | - | 2 | The Vigil Unending stratagem in the Auric Champions detac... |
| `lookup_martial_philosopher_cost` | lookup | yes | yes | - | - | 30 | The Martial Philosopher enhancement has a points cost of ... |
| `lookup_panoptispex_cost` | lookup | yes | yes | - | - | 5 | The Panoptispex enhancement costs 5 points in the Shield ... |
| `lookup_shoulder_phase` | lookup | NO | yes | - | - | movement | The Shoulder the Mantle stratagem is used during the **Mo... |
| `lookup_dropship_transport` | lookup | yes | yes | - | - | 12 | The Orion Assault Dropship is listed as having a single m... |
| `count_roster_adeptus_custodes` | count | yes | yes | - | - | 31 | Adeptus Custodes can field 31 units. |
| `count_roster_necrons` | count | yes | yes | - | - | 64 | According to Wahapedia, the Necrons can field 64 units – ... |
| `count_roster_orks` | count | yes | yes | - | - | 87 | Orks can field 87 units. |
| `count_roster_tyranids` | count | yes | yes | - | - | 57 | According to Wahapedia’s Warhammer 40,000 10th‑edition da... |
| `count_roster_leagues_of_votann` | count | yes | yes | - | - | 22 | The Leagues of Votann can field 22 units. |
| `count_keyword_psyker` | count | yes | yes | - | - | 143 | 143 units have the Psyker keyword. |
| `count_keyword_monster` | count | yes | yes | - | - | 104 | 104 units have the Monster keyword. — Wahapedia |
| `count_keyword_aircraft` | count | yes | yes | - | - | 95 | 95 units have the Aircraft keyword. |
| `count_keyword_fortification` | count | yes | yes | - | - | 45 | There are 45 units that have the Fortification keyword. |
| `count_ability_feel_no_pain` | count | yes | yes | - | - | 116 | According to the data, 116 units have the Feel No Pain ab... |
| `count_ability_deep_strike` | count | yes | yes | - | - | 356 | According to the data, 356 units have the Deep Strike abi... |
| `count_ability_lone_operative` | count | yes | yes | - | - | 52 | 52 units have the ability Lone Operative. |
| `count_ability_scouts` | count | yes | yes | - | - | 110 | According to the data, 110 units have the Scouts ability. |
| `count_shield_host_stratagems` | count | NO | yes | - | - | 6 | The Shield Host detachment has 6 stratagems. |
| `count_voyagers_in_darkness_stratagems` | count | yes | yes | - | - | 4 | The Voyagers in Darkness detachment has 4 stratagems. |
| `count_blood_legion_rules` | count | yes | yes | - | - | 2 | The Blood Legion detachment includes 2 rules. |
| `count_eldritch_raiders_rules` | count | yes | yes | - | - | 2 | The Eldritch Raiders detachment has 2 rules. |
| `count_black_ship_guardians_enhancements` | count | yes | yes | - | - | 2 | The Black Ship Guardians detachment has 2 enhancements. |
| `count_factions` | count | yes | NO | - | 28 | 26 | There are 28 factions in the rules data. |
| `compare_angron_warboss_toughness` | compare | yes | yes | - | - | yes | Yes. Angron’s toughness is 11, while Warboss’s toughness ... |
| `compare_warboss_tyrant_toughness` | compare | yes | yes | - | - | no | No |
| `compare_immortals_lychguard_toughness` | compare | yes | yes | - | - | yes | Yes |
| `compare_warriors_immortals_toughness` | compare | yes | yes | - | - | no | No |
| `compare_wraithknight_avatar_wounds` | compare | yes | NO | - | - | yes | No, Wraithknight has more wounds than Avatar of Khaine. |
| `compare_tyrant_guilliman_points` | compare | yes | yes | - | - | yes | Yes, Hive Tyrant costs fewer points than Roboute Guilliman. |
| `compare_trazyn_warboss_points` | compare | yes | yes | - | - | yes | Yes |
| `compare_psyker_monster_keyword` | compare | yes | yes | - | - | yes | Yes, there are more units with the Psyker keyword than wi... |
| `compare_higher_wounds` | compare | NO | NO | - | - | 5 |  |
| `compare_higher_toughness` | compare | yes | yes | - | - | 11 | Angron has a toughness of 11 (see https://wahapedia.ru/wh... |
| `refuse_invented_unit_toughness` | refuse | yes | - | NO | - | None |  |
| `refuse_invented_unit_points` | refuse | yes | - | NO | - | None |  |
| `refuse_invented_stratagem` | refuse | yes | - | NO | - | None |  |
| `refuse_invented_faction` | refuse | yes | - | yes | - | None | No faction named "Squats" exists in the provided data, so... |
| `refuse_invented_keyword` | refuse | yes | - | NO | - | None | There is **1 unit** that has the Cavalry keyword—specific... |
| `refuse_missing_stat` | refuse | - | - | NO | - | None | I couldn't locate any reference to a "Warp Charge" value ... |
| `refuse_other_edition` | refuse | - | - | NO | - | None | Angron’s toughness was listed as 11 in the 9th‑edition da... |
| `refuse_future_edition` | refuse | - | - | NO | - | None | I need to find the specific points cost for an Intercesso... |
| `refuse_out_of_scope_price` | refuse | - | - | NO | - | None |  |
