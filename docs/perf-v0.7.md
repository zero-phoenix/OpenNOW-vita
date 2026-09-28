# OpenNOW Vita v0.7 — performance notes (megaplan Fase 6)

This file is the measurement plan and results table for the v0.7 UI rework. It follows the
Halyard rule this project adopted: a performance claim ships with the measurement that backs it,
or it does not ship as a claim.

**Status: the hardware cells are pending.** Everything below is committed code that builds green;
the numbers get filled in from `frame_stats.log` on a real Vita (protocol below) before the
v0.7.0 release is tagged. Desktop/Vita3K numbers are not a substitute — a desktop CPU hides the
costs the Vita pays for.

## What changed in v0.7

| Change | Expected effect |
|---|---|
| Shared theme module, focus model moved to `opennow-core` (F1/F5) | None. Code moved, palette bytes identical. |
| Performance HUD + in-stream pause menu (F2/F3) | Only while visible; HUD parsing is throttled to `hud_refresh_ms` (250–1000 ms), never per frame. |
| `ui.rs` split into screen modules (F5) | None. Move, not rewrite. |
| Dynamic frame budget (F6) | Idle menu screens drop from ~60 to ~30 painted fps when there is no input and nothing animating. Per-frame cost unchanged; per-second UI burn roughly halves in exactly the states where nothing is happening. |

## How F6 decides a frame is idle

`src/shell/mod.rs` (loop tail): a frame gets the 33 ms budget only when **all** of these hold:

- no egui events arrived this frame (no touch, no button, no text input);
- not streaming, not in any spinner/animated state (splash, catalog load, session setup,
  device-code polling);
- the splash has fully faded.

Anything that happens — a touch, a button, the stream starting — wakes the loop early and the
full 16.7 ms budget applies again. The states that change their own content never idle.

## Measurement protocol (real Vita)

1. Flash the **v0.6.1** VPK (tag `v0.6.1`, commit `167fc70`). Boot, let the splash finish,
   leave the console untouched on the catalog screen for **3 minutes**.
2. Pull `ux0:data/opennow-vita/frame_stats.log`. From the `frame stats (…s):` blocks record:
   painted fps, iterations, `avg per painted frame` line (build_ui / tessellate / present).
   Then delete the file and repeat the streaming scenario: 5 minutes in a session with the HUD
   on (default refresh 500 ms), record the same plus the count of `slow frame:` and
   `long gap:` lines.
3. Flash the v0.7.0 build and repeat both scenarios.

One log block is written every 2 s (`FRAME_STATS_INTERVAL`); the file is reset at each app
launch, so a scenario run starts from a clean file automatically.

## Results table

| Scenario | Metric | v0.6.1 (before) | v0.7.0 (after) | Criterion |
|---|---|---|---|---|
| Idle catalog, 3 min | painted fps | PENDING | PENDING | ≈60 before, ≈30 after |
| Idle catalog, 3 min | avg present ms/frame | PENDING | PENDING | unchanged (±20%) |
| Idle catalog, 3 min | avg build_ui ms/frame | PENDING | PENDING | unchanged (±20%) |
| Streaming, 5 min | painted fps | PENDING | PENDING | ≥ before (stream never idles) |
| Streaming, 5 min | `slow frame:` count | PENDING | PENDING | not worse than before |
| Streaming, 5 min | `long gap:` count | PENDING | PENDING | not worse than before |
| Pause menu open mid-stream | HUD fps reading drops | PENDING | PENDING | H3: A/V keeps playing |
| 20 mode changes | `control profile ->` log lines | n/a | PENDING | H4: 20 lines, ≤2 presses per change, 0 accidental |

H3/H4 are the megaplan's popperian tests: the pause menu must open without cutting audio/video,
and twenty manual JUEGO|PC switches must show at most two presses each and zero accidental taps
in `ux0:data/opennow-vita/logs/opennow_latest.log`.

## Known non-regressions

- The F5 focus change is behavioural and documented in the CHANGELOG: the server picker now
  wraps at the ends instead of clamping (all list arithmetic now lives in the tested
  `FocusList`). This is deliberate, not drift.
- The game control profile still forwards every control to the title — including Select
  (`core` test `the_game_profile_forwards_every_control`).
