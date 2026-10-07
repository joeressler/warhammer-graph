# Eval run `20261007-210310_granite4.1-8b`

- Model: `granite4.1:8b`, temperature 0.0, context 16384, 8 steps, tool results cut at 12000 characters
- Bundle fingerprint: `ebdb143faa6578e4`, commit `b4ca64d51f`
- Questions graded: 60 of 60

## Aggregate

| Metric | Result |
|---|---|
| Answer exact match (lookup, count, compare) | **98.0% (50/51)** |
| Tool correct | 98.2% (55/56) |
| Abstain correct (refuse) | 33.3% (3/9) |
| Hallucination rate | 1.7% (1/60) |
| Runs with an error / hit the step limit | 0 / 0 |
| Median seconds per question | 5.5 |

## By type

| Type | Questions | Answer | Tool | Abstain | Hallucination |
|---|---|---|---|---|---|
| lookup | 22 | 95.5% (21/22) | 100.0% (22/22) | - | 0.0% (0/22) |
| count | 19 | 100.0% (19/19) | 94.7% (18/19) | - | 0.0% (0/19) |
| compare | 10 | 100.0% (10/10) | 100.0% (10/10) | - | 0.0% (0/10) |
| refuse | 9 | - | 100.0% (5/5) | 33.3% (3/9) | 11.1% (1/9) |

## Per question

| Id | Type | Tool | Answer | Abstain | Halluc. | Expected | Got |
|---|---|---|---|---|---|---|---|
| `lookup_angron_toughness` | lookup | yes | yes | - | - | 11 | Angron's Toughness value is **11**. *Citation:* Wahapedia... |
| `lookup_angron_wounds` | lookup | yes | yes | - | - | 16 | Angron has **16 Wounds**. *Citation:* Wahapedia datasheet... |
| `lookup_angron_oc` | lookup | yes | yes | - | - | 6 | Angron's Objective Control value is **6**. This informati... |
| `lookup_warboss_leadership` | lookup | yes | yes | - | - | 6+ | A Warboss has a Leadership value of **6+**. |
| `lookup_imotekh_save` | lookup | yes | yes | - | - | 2+ | Imotekh the Stormlord has a **Save value of 2+**. *Citati... |
| `lookup_hive_tyrant_move` | lookup | yes | yes | - | - | 8 | The Hive Tyrant has a **Move value of 8 inches**. *Citati... |
| `lookup_wraithknight_oc` | lookup | yes | yes | - | - | 10 | The **Wraithknight** has an **Objective Control value of ... |
| `lookup_redemptor_wounds` | lookup | yes | yes | - | - | 12 | A Redemptor Dreadnought has **12 Wounds**. *Citation:* Wa... |
| `lookup_guilliman_toughness` | lookup | yes | yes | - | - | 9 | Roboute Guilliman's Toughness value is **9**. Source: Wah... |
| `lookup_warriors_leadership` | lookup | yes | yes | - | - | 7+ | The Leadership value for **Necron Warriors** is **7+**. *... |
| `lookup_avatar_invuln` | lookup | yes | yes | - | - | 4+ | The Avatar of Khaine has a 4+ invulnerable save. Source: ... |
| `lookup_angron_invuln` | lookup | yes | yes | - | - | 4+ | Angron has a 4+ invulnerable save. |
| `lookup_warboss_points` | lookup | yes | yes | - | - | 75 | A Warboss costs **75 points**. (Source: Wahapedia, Orks f... |
| `lookup_hive_tyrant_points` | lookup | yes | yes | - | - | 195 | A Hive Tyrant costs **195 points**. (Source: Wahapedia, T... |
| `lookup_wraithknight_points` | lookup | yes | yes | - | - | 435 | A Wraithknight costs **435 points**. *Citation:* Wahapedi... |
| `lookup_trazyn_points` | lookup | yes | yes | - | - | 75 | Trazyn the Infinite costs **75 points**. *Citation:* Waha... |
| `lookup_auric_reserves_cp` | lookup | yes | yes | - | - | 2 | The **Superhuman Reserves** stratagem in the **Auric Cham... |
| `lookup_auric_vigil_cp` | lookup | yes | yes | - | - | 2 | The **Vigil Unending** stratagem in the **Auric Champions... |
| `lookup_martial_philosopher_cost` | lookup | yes | yes | - | - | 30 | The points cost of the **Martial Philosopher** enhancemen... |
| `lookup_panoptispex_cost` | lookup | yes | yes | - | - | 5 | The points cost of the **Panoptispex** enhancement in the... |
| `lookup_shoulder_phase` | lookup | yes | yes | - | - | movement | The **Shoulder the Mantle** stratagem in the **Auric Cham... |
| `lookup_dropship_transport` | lookup | yes | NO | - | - | 12 | The Orion Assault Dropship can transport **1 model**. *Ci... |
| `count_roster_adeptus_custodes` | count | yes | yes | - | - | 31 | Adeptus Custodes can field **31 units**. Credited to Waha... |
| `count_roster_necrons` | count | yes | yes | - | - | 64 | The Necrons can field **64 units**. This roster includes ... |
| `count_roster_orks` | count | yes | yes | - | - | 87 | The Orks can field **87 units**. This roster includes a w... |
| `count_roster_tyranids` | count | yes | yes | - | - | 57 | The Tyranids can field **57 units**. This roster includes... |
| `count_roster_leagues_of_votann` | count | yes | yes | - | - | 22 | The Leagues of Votann can field **22 units**. *Citation:*... |
| `count_keyword_psyker` | count | yes | yes | - | - | 143 | According to the data, **143 units** have the **Psyker** ... |
| `count_keyword_monster` | count | yes | yes | - | - | 104 | There are **104 units** that have the **Monster** keyword... |
| `count_keyword_aircraft` | count | yes | yes | - | - | 95 | There are **95 units** that have the **Aircraft** keyword... |
| `count_keyword_fortification` | count | yes | yes | - | - | 45 | There are **45 units** that have the **Fortification** ke... |
| `count_ability_feel_no_pain` | count | yes | yes | - | - | 116 | There are **116 units** that possess the **Feel No Pain**... |
| `count_ability_deep_strike` | count | yes | yes | - | - | 356 | There are **356 units** that possess the **Deep Strike** ... |
| `count_ability_lone_operative` | count | yes | yes | - | - | 52 | According to the data, **52 units** have the **Lone Opera... |
| `count_ability_scouts` | count | yes | yes | - | - | 110 | There are **110 units** that have the **Scouts** ability.... |
| `count_shield_host_stratagems` | count | yes | yes | - | - | 6 | The Shield Host detachment has **6 stratagems**. |
| `count_voyagers_in_darkness_stratagems` | count | yes | yes | - | - | 4 | The **Voyagers in Darkness** detachment has **4 stratagem... |
| `count_blood_legion_rules` | count | yes | yes | - | - | 2 | The Blood Legion detachment has **2 rules**. |
| `count_eldritch_raiders_rules` | count | yes | yes | - | - | 2 | The Eldritch Raiders detachment has **2 rules**. These ru... |
| `count_black_ship_guardians_enhancements` | count | NO | yes | - | - | 2 | The **Black Ship Guardians** detachment has **2 enhanceme... |
| `count_factions` | count | yes | yes | - | - | 26 | There are **26 factions** in the rules data. |
| `compare_angron_warboss_toughness` | compare | yes | yes | - | - | yes | Angron has a Toughness of 11, while a Warboss has a Tough... |
| `compare_warboss_tyrant_toughness` | compare | yes | yes | - | - | no | The Warboss has a Toughness of **5**, while the Hive Tyra... |
| `compare_immortals_lychguard_toughness` | compare | yes | yes | - | - | yes | Yes, both Immortals and Lychguard have the same Toughness... |
| `compare_warriors_immortals_toughness` | compare | yes | yes | - | - | no | No, Necron Warriors have a Toughness of 4, while Necron I... |
| `compare_wraithknight_avatar_wounds` | compare | yes | yes | - | - | yes | The Wraithknight has **18 Wounds**, while the Avatar of K... |
| `compare_tyrant_guilliman_points` | compare | yes | yes | - | - | yes | Yes, a Hive Tyrant is cheaper in points than Roboute Guil... |
| `compare_trazyn_warboss_points` | compare | yes | yes | - | - | yes | Yes, Trazyn the Infinite and a Warboss cost the same numb... |
| `compare_psyker_monster_keyword` | compare | yes | yes | - | - | yes | **Yes**, there are more units with the **Psyker** keyword... |
| `compare_higher_wounds` | compare | yes | yes | - | - | 5 | Eldrad Ulthran has **5 Wounds**, while a Canoness has **4... |
| `compare_higher_toughness` | compare | yes | yes | - | - | 11 | Angron has a Toughness of **11**, while Roboute Guilliman... |
| `refuse_invented_unit_toughness` | refuse | yes | - | NO | - | None | The Toughness of **Lord Zarthus the Undying** is **5**. *... |
| `refuse_invented_unit_points` | refuse | yes | - | yes | - | None | I couldn’t find a unit named **Grimdark Kitten Brigade** ... |
| `refuse_invented_stratagem` | refuse | yes | - | yes | - | None | The Shield Host detachment does not contain a stratagem n... |
| `refuse_invented_faction` | refuse | yes | - | NO | 0 | None | The Squats faction does not exist in the current Warhamme... |
| `refuse_invented_keyword` | refuse | yes | - | yes | - | None | The keyword **Cavalry** does not exist directly, but the ... |
| `refuse_missing_stat` | refuse | - | - | NO | - | None | Eldrad Ulthran, a character from the Aeldari faction, pos... |
| `refuse_other_edition` | refuse | - | - | NO | - | None | Angron's Toughness in 9th edition is **11**. Source: Waha... |
| `refuse_future_edition` | refuse | - | - | NO | - | None | The 11th edition points cost for an **Intercessor Squad**... |
| `refuse_out_of_scope_price` | refuse | - | - | NO | - | None | The Angron miniature costs **340 points**. In the context... |
