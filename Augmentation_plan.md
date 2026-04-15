# Augmenting Redberry Persona Engine

This plan covers implementing the structural enhancements for the persona logic to make Redberry significantly more reactive and galvanizing.

## Proposed Changes

### Redberry Core & Tracking
- **Fatigue Tracking:** Introduce a basic `FatigueStats` struct in `redberry_core` (or the `PromptAnalysis` struct directly) holding `consecutive_vague_prompts: u32`.
- Update the CLI / Context Cache to return the fatigue multiplier if multiple prompts in the same session fail validations.

---
### redberry-persona/src/lib.rs (Personality Engine)

#### [MODIFY] PersonalityEngine
- **Entity Injection:** Whenever it selects a mock template from TOML, it will scan for `{{entity}}`. If found, it will extract an entity from `analysis.decomposition.entities` and inject it cleanly (e.g. `mockery.replace("{{entity}}", entity)`).
- **Fatigue Escalation Execution:** Route the decision logic to override standard verdicts with "Severe Fatigue" verdicts if `consecutive_vague_prompts >= 3`.

#### [MODIFY] templates.toml
- Modify existing strings to use `{{entity}}` dynamically (e.g. `Oh, another half-baked thought about {{entity}}?`).
- Restructure TOML to hold escalation paths `[fatigue.level_3]`.

#### [MODIFY] calibration.rs
- Ensure `sass_level` natively dictates entirely different arrays of responses, tying `config.sass_level` directly into the TOML category routing.

## Open Questions

> [!WARNING]
> To properly track fatigue (Option 2), we need to track consecutive bad prompts across a session. Since the CLI runs ephemerally, fatigue won't trigger there unless you pass 3 bad clauses in *one* prompt, OR we write fatigue metadata directly to the SQLite `context.db` cache. Are you comfortable with writing `consecutive_bad_prompts` as metadata directly to the SQLite table for persistent tracking across CLI runs?

## Verification Plan

- Run 3 consecutive vague test prompts manually in the CLI. SQLite should increment the fatigue counter. Upon the 3rd fail, we verify it hits the extreme escalating template from `[fatigue.level_3]`.
- Verify the entity replacement by sending an empty string prompt with one tech word: "fix React". Redberry should reply mentioning React dynamically.
