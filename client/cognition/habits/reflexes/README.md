# Habits Reflexes

This directory holds reflex-layer policy for fast micro-routine promotion from habits.

- Runtime routines are stored in `state/client/cognition/adaptive/reflex/routines.json`.
- Use `node client/cognition/habits/scripts/reflex_habit_bridge.ts sync` to promote eligible habits.
- Use `node client/cognition/habits/scripts/reflex_habit_bridge.ts gc` to degrade/disable stale reflex routines.
