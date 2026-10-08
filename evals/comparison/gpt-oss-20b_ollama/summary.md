# Eval run `20261007-231252_gpt-oss-20b`

- Model: `gpt-oss:20b` on ollama, temperature 0.0, context 16384, 8 steps, tool results cut at 12000 characters
- Bundle fingerprint: `ebdb143faa6578e4`, commit `f4d7ff26d6`
- Questions graded: 60 of 60

## Aggregate

| Metric | Result |
|---|---|
| Answer exact match (lookup, count, compare) | **94.1% (48/51)** |
| Tool correct | 98.2% (55/56) |
| Abstain correct (refuse) | 55.6% (5/9) |
| Hallucination rate | 1.7% (1/60) |
| Runs with an error / hit the step limit | 2 / 1 |
| Median seconds per question | 5.3 |

## By type

| Type | Questions | Answer | Tool | Abstain | Hallucination |
|---|---|---|---|---|---|
| lookup | 22 | 100.0% (22/22) | 100.0% (22/22) | - | 0.0% (0/22) |
| count | 19 | 94.7% (18/19) | 100.0% (19/19) | - | 5.3% (1/19) |
| compare | 10 | 80.0% (8/10) | 90.0% (9/10) | - | 0.0% (0/10) |
| refuse | 9 | - | 100.0% (5/5) | 55.6% (5/9) | 0.0% (0/9) |

## Per question

| Id | Type | Tool | Answer | Abstain | Halluc. | Expected | Got |
|---|---|---|---|---|---|---|---|
| `lookup_angron_toughness` | lookup | yes | yes | - | - | 11 | Angron’s Toughness value is **11**【assistant to=functions... |
| `lookup_angron_wounds` | lookup | yes | yes | - | - | 16 | Angron has **16 wounds**【assistant to=functions.get_unit ... |
| `lookup_angron_oc` | lookup | yes | yes | - | - | 6 | Angron’s Objective Control value is **6**【get_unit】. |
| `lookup_warboss_leadership` | lookup | yes | yes | - | - | 6+ | The Warboss has a Leadership characteristic of **6+**【ass... |
| `lookup_imotekh_save` | lookup | yes | yes | - | - | 2+ | Imotekh the Stormlord’s normal save is **2+**【assistant t... |
| `lookup_hive_tyrant_move` | lookup | yes | yes | - | - | 8 | The Hive Tyrant moves **8 inches**. *Source: Wahapedia – ... |
| `lookup_wraithknight_oc` | lookup | yes | yes | - | - | 10 | A Wraithknight has an **Objective Control value of 10**【g... |
| `lookup_redemptor_wounds` | lookup | yes | yes | - | - | 12 | The Redemptor Dreadnought has **12 wounds**【assistant to=... |
| `lookup_guilliman_toughness` | lookup | yes | yes | - | - | 9 | Roboute Guilliman’s Toughness value is **9**【assistant to... |
| `lookup_warriors_leadership` | lookup | yes | yes | - | - | 7+ | The Necron Warriors have a Leadership value of **7+**【get... |
| `lookup_avatar_invuln` | lookup | yes | yes | - | - | 4+ | The Avatar of Khaine has an **invulnerable save of 4+**【a... |
| `lookup_angron_invuln` | lookup | yes | yes | - | - | 4+ | Angron’s datasheet lists an **invulnerable save of 4+**【a... |
| `lookup_warboss_points` | lookup | yes | yes | - | - | 75 | A Warboss costs **75 points** in a 10th‑edition army list... |
| `lookup_hive_tyrant_points` | lookup | yes | yes | - | - | 195 | The Hive Tyrant costs **195 points** in the 10th‑edition ... |
| `lookup_wraithknight_points` | lookup | yes | yes | - | - | 435 | The Wraithknight costs **435 points**【assistant to=functi... |
| `lookup_trazyn_points` | lookup | yes | yes | - | - | 75 | Trazyn the Infinite costs **75 points** in a 10th‑edition... |
| `lookup_auric_reserves_cp` | lookup | yes | yes | - | - | 2 | The **Superhuman Reserves** stratagem in the Auric Champi... |
| `lookup_auric_vigil_cp` | lookup | yes | yes | - | - | 2 | The **Vigil Unending** stratagem in the Auric Champions d... |
| `lookup_martial_philosopher_cost` | lookup | yes | yes | - | - | 30 | The **Martial Philosopher** enhancement in the Auric Cham... |
| `lookup_panoptispex_cost` | lookup | yes | yes | - | - | 5 | The **Panoptispex** enhancement for the Shield Host detac... |
| `lookup_shoulder_phase` | lookup | yes | yes | - | - | movement | The **“Shoulder the Mantle”** stratagem for the Auric Cha... |
| `lookup_dropship_transport` | lookup | yes | yes | - | - | 12 | The Orion Assault Dropship can carry **12 Adeptus Custode... |
| `count_roster_adeptus_custodes` | count | yes | yes | - | - | 31 | Adeptus Custodes can field **31 units** in the 10th‑editi... |
| `count_roster_necrons` | count | yes | yes | - | - | 64 | The Necrons can field **64 units** in total. (Information... |
| `count_roster_orks` | count | yes | yes | - | - | 87 | The Orks can field **87 units** in total. (See the `summa... |
| `count_roster_tyranids` | count | yes | yes | - | - | 57 | Tyranids can field **57 units** in total. (Information fr... |
| `count_roster_leagues_of_votann` | count | yes | yes | - | - | 22 | The Leagues of Votann can field **22 units** in total. *(... |
| `count_keyword_psyker` | count | yes | yes | - | - | 143 | According to Wahapedia’s 10th‑edition data, **143 units**... |
| `count_keyword_monster` | count | yes | yes | - | - | 104 | There are **104 units** that carry the **Monster** keyword. |
| `count_keyword_aircraft` | count | yes | yes | - | - | 95 | According to Wahapedia’s data, **95 units** carry the Air... |
| `count_keyword_fortification` | count | yes | yes | - | - | 45 | According to Wahapedia, **45 units** carry the Fortificat... |
| `count_ability_feel_no_pain` | count | yes | yes | - | - | 116 | According to Wahapedia, **116 units** have the “Feel No P... |
| `count_ability_deep_strike` | count | yes | yes | - | - | 356 | According to Wahapedia, **356 units** have the ability **... |
| `count_ability_lone_operative` | count | yes | yes | - | - | 52 | According to Wahapedia, **52 units** have the “Lone Opera... |
| `count_ability_scouts` | count | yes | yes | - | - | 110 | According to Wahapedia’s data, **110 units** have the abi... |
| `count_shield_host_stratagems` | count | yes | yes | - | - | 6 | The Shield Host detachment has **6 stratagems**【assistant... |
| `count_voyagers_in_darkness_stratagems` | count | yes | yes | - | - | 4 | The Voyagers in Darkness detachment has **4 stratagems**【... |
| `count_blood_legion_rules` | count | yes | yes | - | - | 2 | The Blood Legion detachment has **2 rules**【assistant to=... |
| `count_eldritch_raiders_rules` | count | yes | yes | - | - | 2 | The Eldritch Raiders detachment has **2 rules**【get_detac... |
| `count_black_ship_guardians_enhancements` | count | yes | yes | - | - | 2 | The Black Ship Guardians detachment has **2 enhancements*... |
| `count_factions` | count | yes | NO | - | 27 | 26 | There are **27 factions** represented in the rules data. |
| `compare_angron_warboss_toughness` | compare | NO | NO | - | - | yes |  |
| `compare_warboss_tyrant_toughness` | compare | yes | yes | - | - | no | No. The Warboss has a Toughness of **5** (10ed:datasheet:... |
| `compare_immortals_lychguard_toughness` | compare | yes | yes | - | - | yes | Yes. Both the Immortals and the Lychguard have a Toughnes... |
| `compare_warriors_immortals_toughness` | compare | yes | yes | - | - | no | No. Necron Warriors have a Toughness of 4, while Immortal... |
| `compare_wraithknight_avatar_wounds` | compare | yes | yes | - | - | yes | Yes. The Wraithknight has 18 wounds, while the Avatar of ... |
| `compare_tyrant_guilliman_points` | compare | yes | NO | - | - | yes | No. |
| `compare_trazyn_warboss_points` | compare | yes | yes | - | - | yes | Yes. Both Trazyn the Infinite and a Warboss cost 75 points. |
| `compare_psyker_monster_keyword` | compare | yes | yes | - | - | yes | Yes. There are 143 units with the Psyker keyword, compare... |
| `compare_higher_wounds` | compare | yes | yes | - | - | 5 | The higher Wounds value is **Eldrad Ulthran**, who has **... |
| `compare_higher_toughness` | compare | yes | yes | - | - | 11 | **Toughness values** / Unit / Toughness / /------/-------... |
| `refuse_invented_unit_toughness` | refuse | yes | - | yes | - | None | I couldn’t find a datasheet named “Lord Zarthus the Undyi... |
| `refuse_invented_unit_points` | refuse | yes | - | NO | - | None |  |
| `refuse_invented_stratagem` | refuse | yes | - | yes | - | None | I couldn’t find a “Banana Strike” stratagem in the Shield... |
| `refuse_invented_faction` | refuse | yes | - | yes | - | None | I couldn’t find a faction called “Squats” in the 10th‑edi... |
| `refuse_invented_keyword` | refuse | yes | - | NO | - | None | There is **1 unit** that carries the “Thunderwolf Cavalry... |
| `refuse_missing_stat` | refuse | - | - | NO | - | None | ERROR ResponseError: error parsing tool call: raw='{"kind... |
| `refuse_other_edition` | refuse | - | - | NO | - | None | ERROR TimeoutError: |
| `refuse_future_edition` | refuse | - | - | yes | - | None | I’m sorry, but the Wahapedia data set I have access to on... |
| `refuse_out_of_scope_price` | refuse | - | - | yes | - | None | The Wahapedia datasheet for Angron lists a **cost of 340 ... |
