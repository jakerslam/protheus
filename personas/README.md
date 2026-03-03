# Personas

This directory stores internal operator lenses used for planning, audits, and decision pressure-testing.

## Structure

- `personas/<name>/profile.md` - background, strengths, failure modes, communication style
- `personas/<name>/decision_lens.md` - tactical decision filters and default pushback
- `personas/<name>/strategic_lens.md` - long-horizon mission and scale framing
- `personas/<name>/lens.md` - legacy compatibility shim (mirrors decision lens)
- `personas/<name>/correspondence.md` - timestamped notes and decision history
- `personas/<name>/emotion_lens.md` (optional) - emotional response patterns used to enrich lens output

## Operating Rules

- Use personas as analysis lenses, not as authority replacement.
- Use `protheus lens <persona> --lens=decision|strategic|full "<query>"` for targeted mode selection.
- Record significant decisions in correspondence logs.
- Keep language concise, technical, and auditable.
- Do not put secrets in this directory.

## Current Personas

- `aarav_singh`
- `jay_haslam`
- `li_wei`
- `priya_venkatesh`
- `rohan_kapoor`
- `vikram_menon`
