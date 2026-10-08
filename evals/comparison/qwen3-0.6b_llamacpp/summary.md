# Eval run `20261007-224847_qwen3-0.6b_llamacpp`

- Model: `qwen3:0.6b` on llamacpp, temperature 0.0, context 16384, 8 steps, tool results cut at 12000 characters
- Bundle fingerprint: `ebdb143faa6578e4`, commit `f4d7ff26d6`
- Questions graded: 60 of 60

## Aggregate

| Metric | Result |
|---|---|
| Answer exact match (lookup, count, compare) | **76.5% (39/51)** |
| Tool correct | 83.9% (47/56) |
| Abstain correct (refuse) | 33.3% (3/9) |
| Hallucination rate | 3.3% (2/60) |
| Runs with an error / hit the step limit | 0 / 0 |
| Median seconds per question | 3.8 |

## By type

| Type | Questions | Answer | Tool | Abstain | Hallucination |
|---|---|---|---|---|---|
| lookup | 22 | 90.9% (20/22) | 90.9% (20/22) | - | 0.0% (0/22) |
| count | 19 | 73.7% (14/19) | 78.9% (15/19) | - | 10.5% (2/19) |
| compare | 10 | 50.0% (5/10) | 70.0% (7/10) | - | 0.0% (0/10) |
| refuse | 9 | - | 100.0% (5/5) | 33.3% (3/9) | 0.0% (0/9) |

## Per question

| Id | Type | Tool | Answer | Abstain | Halluc. | Expected | Got |
|---|---|---|---|---|---|---|---|
| `lookup_angron_toughness` | lookup | yes | yes | - | - | 11 | The Toughness value for Angron is 11. |
| `lookup_angron_wounds` | lookup | yes | yes | - | - | 16 | Angron has 16 wounds as specified in the datasheet. |
| `lookup_angron_oc` | lookup | yes | yes | - | - | 6 | The Objective Control value for Angron is 6. |
| `lookup_warboss_leadership` | lookup | NO | NO | - | - | 6+ | The value of a Warboss's Leadership is specified in the u... |
| `lookup_imotekh_save` | lookup | yes | yes | - | - | 2+ | Imotekh the Stormlord's Save value is 2+. |
| `lookup_hive_tyrant_move` | lookup | NO | NO | - | - | 8 | The Move value for a Hive Tyrant is not explicitly listed... |
| `lookup_wraithknight_oc` | lookup | yes | yes | - | - | 10 | The Wraithknight's Objective Control value is 10. |
| `lookup_redemptor_wounds` | lookup | yes | yes | - | - | 12 | The Redemptor Dreadnought has a Wounds value of **12**. |
| `lookup_guilliman_toughness` | lookup | yes | yes | - | - | 9 | The Toughness value for Roboute Guilliman is 9. |
| `lookup_warriors_leadership` | lookup | yes | yes | - | - | 7+ | The Leadership value for Necron Warriors is 7+. |
| `lookup_avatar_invuln` | lookup | yes | yes | - | - | 4+ | The Avatar of Khaine has an invulnerable save of **4+**. |
| `lookup_angron_invuln` | lookup | yes | yes | - | - | 4+ | The invulnerable save for Angron is 4+. |
| `lookup_warboss_points` | lookup | yes | yes | - | - | 75 | The Warboss costs 75 points. |
| `lookup_hive_tyrant_points` | lookup | yes | yes | - | - | 195 | The points cost for a Hive Tyrant is 195. |
| `lookup_wraithknight_points` | lookup | yes | yes | - | - | 435 | The Wraithknight costs 435 points. |
| `lookup_trazyn_points` | lookup | yes | yes | - | - | 75 | The points cost for Trazyn the Infinite is 75. |
| `lookup_auric_reserves_cp` | lookup | yes | yes | - | - | 2 | The Superhuman Reserves stratagem costs 2 command points. |
| `lookup_auric_vigil_cp` | lookup | yes | yes | - | - | 2 | The Vigil Unending stratagem in the Auric Champions detac... |
| `lookup_martial_philosopher_cost` | lookup | yes | yes | - | - | 30 | The points cost of the Martial Philosopher enhancement in... |
| `lookup_panoptispex_cost` | lookup | yes | yes | - | - | 5 | The points cost of the Panoptispex enhancement in the Shi... |
| `lookup_shoulder_phase` | lookup | yes | yes | - | - | movement | The "SHOULDER THE MANTLE" stratagem in the Auric Champion... |
| `lookup_dropship_transport` | lookup | yes | yes | - | - | 12 | The Orion Assault Dropship can transport 12 Adeptus Custo... |
| `count_roster_adeptus_custodes` | count | yes | yes | - | - | 31 | Adeptus Custodes can field 31 units. |
| `count_roster_necrons` | count | yes | yes | - | - | 64 | The Necrons field can field 64 units. |
| `count_roster_orks` | count | NO | NO | - | 5 | 87 | The closest names for the Orks field are Morkanaut, Gorka... |
| `count_roster_tyranids` | count | NO | NO | - | - | 57 | The tools provided do not include a function to list all ... |
| `count_roster_leagues_of_votann` | count | NO | NO | - | - | 22 | The error indicates no units were found for "Leagues of V... |
| `count_keyword_psyker` | count | yes | yes | - | - | 143 | 143 units have the keyword Psyker. |
| `count_keyword_monster` | count | yes | yes | - | - | 104 | 104 units have the Monster keyword. |
| `count_keyword_aircraft` | count | yes | yes | - | - | 95 | 95 units have the Aircraft keyword. |
| `count_keyword_fortification` | count | yes | yes | - | - | 45 | 45 units have the Fortification keyword. |
| `count_ability_feel_no_pain` | count | yes | yes | - | - | 116 | 116 units have the "Feel No Pain" ability. |
| `count_ability_deep_strike` | count | NO | NO | - | - | 356 | The available suggestions for the Deep Strike ability are... |
| `count_ability_lone_operative` | count | yes | yes | - | - | 52 | 52 units have the Lone Operative ability. |
| `count_ability_scouts` | count | yes | yes | - | - | 110 | 110 units have the Scouts ability. |
| `count_shield_host_stratagems` | count | yes | yes | - | - | 6 | The Shield Host detachment has 6 stratagems. |
| `count_voyagers_in_darkness_stratagems` | count | yes | yes | - | - | 4 | The Voyagers in Darkness detachment has 4 stratagems. |
| `count_blood_legion_rules` | count | yes | yes | - | - | 2 | The Blood Legion detachment has 2 rules. |
| `count_eldritch_raiders_rules` | count | yes | yes | - | - | 2 | The Eldritch Raiders detachment has 2 rules. |
| `count_black_ship_guardians_enhancements` | count | yes | yes | - | - | 2 | The Black Ship Guardians detachment has 2 enhancements. |
| `count_factions` | count | yes | NO | - | 20 | 26 | There are 20 factions in the rules data. |
| `compare_angron_warboss_toughness` | compare | yes | yes | - | - | yes | Yes, Angron has higher Toughness than a Warboss. Angron's... |
| `compare_warboss_tyrant_toughness` | compare | yes | NO | - | - | no | Yes, a Warboss's Toughness is higher than a Hive Tyrant's... |
| `compare_immortals_lychguard_toughness` | compare | NO | NO | - | - | yes | The tools returned no information about Immortals or Lych... |
| `compare_warriors_immortals_toughness` | compare | yes | NO | - | - | no | Based on the information provided, both Necron Warriors a... |
| `compare_wraithknight_avatar_wounds` | compare | yes | yes | - | - | yes | Yes, the Wraithknight has more wounds than the Avatar of ... |
| `compare_tyrant_guilliman_points` | compare | yes | yes | - | - | yes | Yes, the Hive Tyrant is cheaper in points than Roboute Gu... |
| `compare_trazyn_warboss_points` | compare | NO | NO | - | - | yes | The tools provided do not include information about the p... |
| `compare_psyker_monster_keyword` | compare | yes | yes | - | - | yes | Yes. There are more units with the keyword "Monster" than... |
| `compare_higher_wounds` | compare | NO | NO | - | - | 5 | The tools do not provide the Wounds value for Eldrad Ulth... |
| `compare_higher_toughness` | compare | yes | yes | - | - | 11 | The higher Toughness value for Angron is 11, and for Robo... |
| `refuse_invented_unit_toughness` | refuse | yes | - | NO | - | None | The available data does not include the Toughness of Lord... |
| `refuse_invented_unit_points` | refuse | yes | - | yes | - | None | The provided tools do not include a function to directly ... |
| `refuse_invented_stratagem` | refuse | yes | - | yes | - | None | The provided data does not list the Banana Strike stratag... |
| `refuse_invented_faction` | refuse | yes | - | yes | - | None | The Squats faction can field **Adepta Sororitas**. The er... |
| `refuse_invented_keyword` | refuse | yes | - | NO | - | None | There is 1 unit with the keyword "Thunderwolf Cavalry." T... |
| `refuse_missing_stat` | refuse | - | - | NO | - | None | The provided data does not include the Warp Charge value ... |
| `refuse_other_edition` | refuse | - | - | NO | - | None | Angron's Toughness in 9th edition is **11**. |
| `refuse_future_edition` | refuse | - | - | NO | - | None | The 11th edition points cost for an Intercessor Squad is ... |
| `refuse_out_of_scope_price` | refuse | - | - | NO | - | None | The Angron miniature costs 340 points in the Warhammer 40... |
