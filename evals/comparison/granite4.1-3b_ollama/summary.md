# Eval run `20261007-230112_granite4.1-3b`

- Model: `granite4.1:3b` on ollama, temperature 0.0, context 16384, 8 steps, tool results cut at 12000 characters
- Bundle fingerprint: `ebdb143faa6578e4`, commit `f4d7ff26d6`
- Questions graded: 60 of 60

## Aggregate

| Metric | Result |
|---|---|
| Answer exact match (lookup, count, compare) | **86.3% (44/51)** |
| Tool correct | 96.4% (54/56) |
| Abstain correct (refuse) | 11.1% (1/9) |
| Hallucination rate | 6.7% (4/60) |
| Runs with an error / hit the step limit | 0 / 0 |
| Median seconds per question | 1.9 |

## By type

| Type | Questions | Answer | Tool | Abstain | Hallucination |
|---|---|---|---|---|---|
| lookup | 22 | 95.5% (21/22) | 100.0% (22/22) | - | 0.0% (0/22) |
| count | 19 | 100.0% (19/19) | 100.0% (19/19) | - | 0.0% (0/19) |
| compare | 10 | 40.0% (4/10) | 90.0% (9/10) | - | 0.0% (0/10) |
| refuse | 9 | - | 80.0% (4/5) | 11.1% (1/9) | 44.4% (4/9) |

## Per question

| Id | Type | Tool | Answer | Abstain | Halluc. | Expected | Got |
|---|---|---|---|---|---|---|---|
| `lookup_angron_toughness` | lookup | yes | yes | - | - | 11 | Angron’s Toughness value is **11**. |
| `lookup_angron_wounds` | lookup | yes | yes | - | - | 16 | Angron’s Wounds value is **16**. |
| `lookup_angron_oc` | lookup | yes | yes | - | - | 6 | Angron’s Objective Control value is **6**. |
| `lookup_warboss_leadership` | lookup | yes | NO | - | - | 6+ | The Warboss (the Warboss keyword) does not have an explic... |
| `lookup_imotekh_save` | lookup | yes | yes | - | - | 2+ | Imotekh the Stormlord has a **2+ save** (and an invulnera... |
| `lookup_hive_tyrant_move` | lookup | yes | yes | - | - | 8 | The Hive Tyrant moves **8 inches** per turn. |
| `lookup_wraithknight_oc` | lookup | yes | yes | - | - | 10 | The Wraithknight’s Objective Control value is **10**. |
| `lookup_redemptor_wounds` | lookup | yes | yes | - | - | 12 | The Redemptor Dreadnought has **12 Wounds**. |
| `lookup_guilliman_toughness` | lookup | yes | yes | - | - | 9 | Roboute Guilliman’s Toughness value is **9**. |
| `lookup_warriors_leadership` | lookup | yes | yes | - | - | 7+ | The Necron Warriors have a Leadership value of **7+**. |
| `lookup_avatar_invuln` | lookup | yes | yes | - | - | 4+ | The Avatar of Khaine has an **invulnerable save of 4+**. |
| `lookup_angron_invuln` | lookup | yes | yes | - | - | 4+ | Angron’s invulnerable save is **4+**. |
| `lookup_warboss_points` | lookup | yes | yes | - | - | 75 | A Warboss costs **75 points**. |
| `lookup_hive_tyrant_points` | lookup | yes | yes | - | - | 195 | A Hive Tyrant costs **195 points**. |
| `lookup_wraithknight_points` | lookup | yes | yes | - | - | 435 | The Wraithknight costs **435 points**. |
| `lookup_trazyn_points` | lookup | yes | yes | - | - | 75 | Trazyn the Infinite costs **75 points**. |
| `lookup_auric_reserves_cp` | lookup | yes | yes | - | - | 2 | The **Superhuman Reserves** stratagem in the Auric Champi... |
| `lookup_auric_vigil_cp` | lookup | yes | yes | - | - | 2 | The **Vigil Unending** stratagem in the Auric Champions d... |
| `lookup_martial_philosopher_cost` | lookup | yes | yes | - | - | 30 | The **Martial Philosopher** enhancement in the Auric Cham... |
| `lookup_panoptispex_cost` | lookup | yes | yes | - | - | 5 | The **Panoptispex** enhancement in the **Shield Host** de... |
| `lookup_shoulder_phase` | lookup | yes | yes | - | - | movement | The **Shoulder the Mantle** stratagem in the Auric Champi... |
| `lookup_dropship_transport` | lookup | yes | yes | - | - | 12 | The Orion Assault Dropship can transport **12 Adeptus Cus... |
| `count_roster_adeptus_custodes` | count | yes | yes | - | - | 31 | The Adeptus Custodes can field **31 distinct units**. |
| `count_roster_necrons` | count | yes | yes | - | - | 64 | The Necrons can field **64 units** in the 10th‑edition Wa... |
| `count_roster_orks` | count | yes | yes | - | - | 87 | The Orks can field **87** different units in the 10th‑edi... |
| `count_roster_tyranids` | count | yes | yes | - | - | 57 | The Tyranids can field **57 units** in the 10th‑edition W... |
| `count_roster_leagues_of_votann` | count | yes | yes | - | - | 22 | The Leagues of Votann can field **22 units**. |
| `count_keyword_psyker` | count | yes | yes | - | - | 143 | **143** units have the **Psyker** keyword in the 10th‑edi... |
| `count_keyword_monster` | count | yes | yes | - | - | 104 | **104 units** have the **Monster** keyword in the 10th‑ed... |
| `count_keyword_aircraft` | count | yes | yes | - | - | 95 | **95 units** have the **Aircraft** keyword in the 10th‑ed... |
| `count_keyword_fortification` | count | yes | yes | - | - | 45 | There are **45** units in the 10th‑edition Warhammer 40,0... |
| `count_ability_feel_no_pain` | count | yes | yes | - | - | 116 | **116 units** have the **Feel No Pain** ability in the 10... |
| `count_ability_deep_strike` | count | yes | yes | - | - | 356 | **356 units** in the 10th‑edition Warhammer 40,000 have t... |
| `count_ability_lone_operative` | count | yes | yes | - | - | 52 | **52** units in the 10th‑edition Warhammer 40,000 have th... |
| `count_ability_scouts` | count | yes | yes | - | - | 110 | There are **110** units in the 10th‑edition Warhammer 40,... |
| `count_shield_host_stratagems` | count | yes | yes | - | - | 6 | The Shield Host detachment includes **6 stratagems**. |
| `count_voyagers_in_darkness_stratagems` | count | yes | yes | - | - | 4 | The **Voyagers in Darkness** detachment includes **4 stra... |
| `count_blood_legion_rules` | count | yes | yes | - | - | 2 | The **Blood Legion** detachment contains **2 rules** (the... |
| `count_eldritch_raiders_rules` | count | yes | yes | - | - | 2 | The Eldritch Raiders detachment includes **2 rules** (the... |
| `count_black_ship_guardians_enhancements` | count | yes | yes | - | - | 2 | The **Black Ship Guardians** detachment includes **2 enha... |
| `count_factions` | count | yes | yes | - | - | 26 | There are **26 factions** in the Warhammer 40,000 10th‑ed... |
| `compare_angron_warboss_toughness` | compare | yes | NO | - | - | yes | no |
| `compare_warboss_tyrant_toughness` | compare | yes | yes | - | - | no | no |
| `compare_immortals_lychguard_toughness` | compare | yes | NO | - | - | yes | no |
| `compare_warriors_immortals_toughness` | compare | NO | yes | - | - | no | no |
| `compare_wraithknight_avatar_wounds` | compare | yes | NO | - | - | yes | no |
| `compare_tyrant_guilliman_points` | compare | yes | NO | - | - | yes | no |
| `compare_trazyn_warboss_points` | compare | yes | NO | - | - | yes | no |
| `compare_psyker_monster_keyword` | compare | yes | NO | - | - | yes | no |
| `compare_higher_wounds` | compare | yes | yes | - | - | 5 | Eldrad Ulthran has **5 Wounds**, while a Canoness has **4... |
| `compare_higher_toughness` | compare | yes | yes | - | - | 11 | Angron has a Toughness of **11**, while Roboute Guilliman... |
| `refuse_invented_unit_toughness` | refuse | yes | - | NO | 4 | None | **Lord Zarthus the Undying** is a **Lord** character in t... |
| `refuse_invented_unit_points` | refuse | NO | - | yes | - | None | The **Grimdark Kitten Brigade** is not a named detachment... |
| `refuse_invented_stratagem` | refuse | yes | - | NO | - | None | The **Banana Strike** stratagem in the **Shield Host** de... |
| `refuse_invented_faction` | refuse | yes | - | NO | 0 | None | The **Squats** are not listed among the factions in the 1... |
| `refuse_invented_keyword` | refuse | yes | - | NO | - | None | There is **1** unit with the keyword **Thunderwolf Cavalr... |
| `refuse_missing_stat` | refuse | - | - | NO | 5 | None | Eldrad Ulthran has **Warp Charge 5**. This is listed in h... |
| `refuse_other_edition` | refuse | - | - | NO | - | None | Angron’s Toughness in the 9th‑edition Warhammer 40,000 da... |
| `refuse_future_edition` | refuse | - | - | NO | - | None | In the 11th‑edition Warhammer 40,000 rules the Intercesso... |
| `refuse_out_of_scope_price` | refuse | - | - | NO | 39.99 | None | The Angron miniature is priced at **$39.99** (USD) in the... |
