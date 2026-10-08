# Eval run `20261007-225338_lfm2.5-8b-a1b-q8-0_llamacpp`

- Model: `lfm2.5:8b-a1b-q8_0` on llamacpp, temperature 0.0, context 16384, 8 steps, tool results cut at 12000 characters
- Bundle fingerprint: `ebdb143faa6578e4`, commit `f4d7ff26d6`
- Questions graded: 60 of 60

## Aggregate

| Metric | Result |
|---|---|
| Answer exact match (lookup, count, compare) | **0.0% (0/51)** |
| Tool correct | 0.0% (0/56) |
| Abstain correct (refuse) | 0.0% (0/9) |
| Hallucination rate | 0.0% (0/60) |
| Runs with an error / hit the step limit | 0 / 0 |
| Median seconds per question | 4.9 |

## By type

| Type | Questions | Answer | Tool | Abstain | Hallucination |
|---|---|---|---|---|---|
| lookup | 22 | 0.0% (0/22) | 0.0% (0/22) | - | 0.0% (0/22) |
| count | 19 | 0.0% (0/19) | 0.0% (0/19) | - | 0.0% (0/19) |
| compare | 10 | 0.0% (0/10) | 0.0% (0/10) | - | 0.0% (0/10) |
| refuse | 9 | - | 0.0% (0/5) | 0.0% (0/9) | 0.0% (0/9) |

## Per question

| Id | Type | Tool | Answer | Abstain | Halluc. | Expected | Got |
|---|---|---|---|---|---|---|---|
| `lookup_angron_toughness` | lookup | NO | NO | - | - | 11 | </tool_call_start/>[get_unit(name='Angron')]</tool_call_e... |
| `lookup_angron_wounds` | lookup | NO | NO | - | - | 16 | </tool_call_start/>[get_unit(name='Angron')]</tool_call_e... |
| `lookup_angron_oc` | lookup | NO | NO | - | - | 6 | </tool_call_start/>[get_unit(name='Angron')]</tool_call_e... |
| `lookup_warboss_leadership` | lookup | NO | NO | - | - | 6+ | </tool_call_start/>[get_unit('Warboss')]</tool_call_end/> |
| `lookup_imotekh_save` | lookup | NO | NO | - | - | 2+ | </tool_call_start/>[get_unit(name='Imotekh the Stormlord'... |
| `lookup_hive_tyrant_move` | lookup | NO | NO | - | - | 8 | </tool_call_start/>[get_unit(name='Hive Tyrant')]</tool_c... |
| `lookup_wraithknight_oc` | lookup | NO | NO | - | - | 10 | </tool_call_start/>[get_unit(unit_name='Wraithknight')]</... |
| `lookup_redemptor_wounds` | lookup | NO | NO | - | - | 12 | </tool_call_start/>[get_unit(name='Redemptor Dreadnought'... |
| `lookup_guilliman_toughness` | lookup | NO | NO | - | - | 9 | </tool_call_start/>[get_unit('Roboute Guilliman')]</tool_... |
| `lookup_warriors_leadership` | lookup | NO | NO | - | - | 7+ | </tool_call_start/>[get_roster(faction='Necron')]</tool_c... |
| `lookup_avatar_invuln` | lookup | NO | NO | - | - | 4+ | </tool_call_start/>[get_roster(name='Avatar of Khaine')]<... |
| `lookup_angron_invuln` | lookup | NO | NO | - | - | 4+ | </tool_call_start/>[get_unit(name='Angron')]</tool_call_e... |
| `lookup_warboss_points` | lookup | NO | NO | - | - | 75 | </tool_call_start/>[get_unit('Warboss')]</tool_call_end/> |
| `lookup_hive_tyrant_points` | lookup | NO | NO | - | - | 195 | </tool_call_start/>[get_unit('Hive Tyrant')]</tool_call_e... |
| `lookup_wraithknight_points` | lookup | NO | NO | - | - | 435 | </tool_call_start/>[get_unit(name='Wraithknight')]</tool_... |
| `lookup_trazyn_points` | lookup | NO | NO | - | - | 75 | </tool_call_start/>[get_unit('Trazyn the Infinite')]</too... |
| `lookup_auric_reserves_cp` | lookup | NO | NO | - | - | 2 | </tool_call_start/>[get_detachment(name='Auric Champions'... |
| `lookup_auric_vigil_cp` | lookup | NO | NO | - | - | 2 | </tool_call_start/>[get_detachment(name='Auric Champions'... |
| `lookup_martial_philosopher_cost` | lookup | NO | NO | - | - | 30 | </tool_call_start/>[get_unit('Martial Philosopher')]</too... |
| `lookup_panoptispex_cost` | lookup | NO | NO | - | - | 5 | </tool_call_start/>[get_roster(faction='Auxilium')]</tool... |
| `lookup_shoulder_phase` | lookup | NO | NO | - | - | movement | </tool_call_start/>[get_roster(name='Auric Champions')]</... |
| `lookup_dropship_transport` | lookup | NO | NO | - | - | 12 | </tool_call_start/>[get_unit(name='Orion Assault Dropship... |
| `count_roster_adeptus_custodes` | count | NO | NO | - | - | 31 | </tool_call_start/>[get_roster(faction='Adeptus Custodes'... |
| `count_roster_necrons` | count | NO | NO | - | - | 64 | </tool_call_start/>[get_roster(faction='Necrons')]</tool_... |
| `count_roster_orks` | count | NO | NO | - | - | 87 | </tool_call_start/>[get_roster(faction='Orks')]</tool_cal... |
| `count_roster_tyranids` | count | NO | NO | - | - | 57 | </tool_call_start/>[get_roster(faction='Tyranids')]</tool... |
| `count_roster_leagues_of_votann` | count | NO | NO | - | - | 22 | </tool_call_start/>[get_roster(faction='Leagues of Votann... |
| `count_keyword_psyker` | count | NO | NO | - | - | 143 | </tool_call_start/>[get_unit(name='Psyker')]</tool_call_e... |
| `count_keyword_monster` | count | NO | NO | - | - | 104 | </tool_call_start/>[get_roster(name='Monster')]</tool_cal... |
| `count_keyword_aircraft` | count | NO | NO | - | - | 95 | I’m not sure which units you’re referring to. Could you s... |
| `count_keyword_fortification` | count | NO | NO | - | - | 45 | </tool_call_start/>[get_unit(name='Fortification')]</tool... |
| `count_ability_feel_no_pain` | count | NO | NO | - | - | 116 | </tool_call_start/>[get_roster(faction='Auxilium')]</tool... |
| `count_ability_deep_strike` | count | NO | NO | - | - | 356 | I cannot determine the number of units that have the Deep... |
| `count_ability_lone_operative` | count | NO | NO | - | - | 52 | </tool_call_start/>[get_roster(faction='Auxiliary')]</too... |
| `count_ability_scouts` | count | NO | NO | - | - | 110 | </tool_call_start/>[get_roster(faction='Imperial Guard')]... |
| `count_shield_host_stratagems` | count | NO | NO | - | - | 6 | </tool_call_start/>[get_detachment(id='Shield Host', name... |
| `count_voyagers_in_darkness_stratagems` | count | NO | NO | - | - | 4 | </tool_call_start/>[get_detachment(id='Voyagers in Darkne... |
| `count_blood_legion_rules` | count | NO | NO | - | - | 2 | </tool_call_start/>[get_detachment(name='Blood Legion')]<... |
| `count_eldritch_raiders_rules` | count | NO | NO | - | - | 2 | </tool_call_start/>[get_unit('Eldritch Raiders')]</tool_c... |
| `count_black_ship_guardians_enhancements` | count | NO | NO | - | - | 2 | </tool_call_start/>[get_unit(name='Black Ship Guardians')... |
| `count_factions` | count | NO | NO | - | - | 26 | The available tools do not include a function to directly... |
| `compare_angron_warboss_toughness` | compare | NO | NO | - | - | yes | </tool_call_start/>[get_unit(name='Angron')]</tool_call_e... |
| `compare_warboss_tyrant_toughness` | compare | NO | NO | - | - | no | </tool_call_start/>[get_unit('Warboss')]</tool_call_end/> |
| `compare_immortals_lychguard_toughness` | compare | NO | NO | - | - | yes | </tool_call_start/>[get_unit(unit_name='Immortals'), get_... |
| `compare_warriors_immortals_toughness` | compare | NO | NO | - | - | no | </tool_call_start/>[get_unit(name='Necron Warrior')]</too... |
| `compare_wraithknight_avatar_wounds` | compare | NO | NO | - | - | yes | </tool_call_start/>[get_unit('Wraithknight')]</tool_call_... |
| `compare_tyrant_guilliman_points` | compare | NO | NO | - | - | yes | </tool_call_start/>[get_unit('Hive Tyrant')]</tool_call_e... |
| `compare_trazyn_warboss_points` | compare | NO | NO | - | - | yes | </tool_call_start/>[get_unit('Trazyn the Infinite')]</too... |
| `compare_psyker_monster_keyword` | compare | NO | NO | - | - | yes | I cannot determine from the available data. |
| `compare_higher_wounds` | compare | NO | NO | - | - | 5 | </tool_call_start/>[get_unit(name='Eldrad Ulthran'), get_... |
| `compare_higher_toughness` | compare | NO | NO | - | - | 11 | </tool_call_start/>[get_unit(name='Angron')]</tool_call_e... |
| `refuse_invented_unit_toughness` | refuse | NO | - | NO | - | None | </tool_call_start/>[get_unit(name='Lord Zarthus the Undyi... |
| `refuse_invented_unit_points` | refuse | NO | - | NO | - | None | </tool_call_start/>[get_unit(name='Grimdark Kitten Brigad... |
| `refuse_invented_stratagem` | refuse | NO | - | NO | - | None | </tool_call_start/>[get_detachment(name='Shield Host')]</... |
| `refuse_invented_faction` | refuse | NO | - | NO | - | None | </tool_call_start/>[get_roster(faction='Squats')]</tool_c... |
| `refuse_invented_keyword` | refuse | NO | - | NO | - | None | </tool_call_start/>[get_roster(faction='Auxiliaries')]</t... |
| `refuse_missing_stat` | refuse | - | - | NO | - | None | </tool_call_start/>[get_unit('Eldrad Ulthran')]</tool_cal... |
| `refuse_other_edition` | refuse | - | - | NO | - | None | </tool_call_start/>[get_unit(name='Angron')]</tool_call_e... |
| `refuse_future_edition` | refuse | - | - | NO | - | None | </tool_call_start/>[get_unit('Intercessor Squad')]</tool_... |
| `refuse_out_of_scope_price` | refuse | - | - | NO | - | None | </tool_call_start/>[get_unit(name='Angron')]</tool_call_e... |
