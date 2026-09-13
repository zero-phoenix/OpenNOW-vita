# Changelog

All notable changes to OpenNOW Vita are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-09-13

### Added

- **PC-Touch Overlay Mod**: use the Vita as a keyboard+mouse for the remote host's desktop
  (e.g. controlling Windows from inside a GeForce NOW Ultimate Install-to-Play session), gated
  behind a new "PC overlay" toggle in Settings so it does not interfere with normal controller
  play:
  - The rear touch panel (device id 2) drives the host mouse cursor as a relative-delta
    trackpad, with a configurable DPI/sensitivity multiplier. NVST's input channel has no
    absolute-position packet, so this is a deliberate trackpad-style relative mapping, not a
    1:1 touch-to-pixel map - documented in `src/input.rs` and `src/gfn/input_protocol.rs`.
  - The front screen exposes seven overlay zones, drawn with egui as semi-transparent
    labelled rectangles at a configurable opacity: Esc (top-left), open native settings
    (top-right), a DPI slider (left edge), left-click and right-click (bottom corners), a
    scroll slider and Enter (right edge). These zones are mutually exclusive with the
    existing `FrontStickZones` L3/R3 corners - when the PC overlay is on, the bottom corners
    are mouse clicks, not sticks; when it's off, `FrontStickZones` behaves exactly as before.
  - Added `INPUT_MOUSE_WHEEL` to the NVST input-channel protocol (`src/gfn/input_protocol.rs`)
    to support scroll, ported from the sibling OpenNOW/OpenNOW-Switch clients' wire format.
  - Physical-button macros, active only while the overlay is on: L trigger (held) halves the
    mouse sensitivity ("sniper mode"), SELECT toggles the on-screen keyboard, D-Pad Up sends
    Win+D, D-Pad Down sends Ctrl+Alt+Del. `SendChord` was extended with a `win: bool` field,
    pairing `KEY_LEFT_WIN` down/up around the chorded key.
  - All of the above (overlay on/off, opacity, DPI sensitivity) are persistent preferences in
    `stream_prefs.rs`, editable from the Settings menu, in all four shipped locales.

### Fixed

- Install-to-Play titles owned on a linked store (e.g. Steam) but also present as an
  Install-to-Play catalog variant could fail to appear or launch correctly: variant selection
  in `catalog.rs` now prefers the owned/`gfn.library.selected` variant instead of blindly
  picking the first numeric-id variant, and `accountLinked` in the CloudMatch session request
  now reflects real per-game ownership instead of being hardcoded `true`.
- "Session limit reached for this device" could fire even right after successfully launching
  the same game from a PC, because the pre-launch zombie-session cleanup only checked the
  pinned zone plus the global entrypoint - a session left open on a *third* zone from an
  earlier run was invisible to cleanup and exhausted every retry. Cleanup now also checks
  every zone already known from region discovery (`build_cleanup_bases()` in `cloudmatch.rs`).

### Changed

- Streaming protocol aligned with OpenNOW desktop: CloudMatch `mediaConnectionInfo` is injected as a remote host ICE candidate, inbound TCP ICE is dropped, and signaling `peerRole` is 1. Keys and mouse stay on the reliable input channel; gamepad does too (the partial-reliable 0x26 path left Vita buttons dead — GFN was not consuming it). Decode fallback is 960x544. A video-stall watchdog sends PLI at 4s and fails the session at 8s. Overlay reports kbps, loss, and RTT. NVST advertises 8-bit depth; CloudMatch `bitDepth` stays `0` (8-bit SDR — NVIDIA's enum, not the bit count). Mid-session bitrate changes send REMB and a fresh NVST blob instead of rewriting a local SDP string. Answer SDP advertises nack/REMB so the rtc interceptor can request retransmits.

### Also fixed (streaming protocol)

- Session create no longer sends CloudMatch `requestedStreamingFeatures.bitDepth: 8`. That value is invalid (`0` = 8-bit SDR, `10` = 10-bit HDR) and NVIDIA answers HTTP 400 with a stub session body.
- Gamepad is sent on `input_channel_v1` again. Routing it exclusively through `input_channel_partially_reliable` (OpenNOW desktop 0x26 packets) made Vita face buttons and sticks a no-op.
- CloudMatch `mediaConnectionInfo` hostnames like `66-22-133-156.cloudmatchbeta.nvidiagrid.net` are decoded to IPv4 before ICE inject. Passing the hostname made `rtc` reject the candidate (`failed to parse address`). Signaling ports (322/443, usage 14) are not injected: that candidate panicked tokio (`EINVAL`) and froze ICE at checking.
- Catalog GraphQL now resolves `vpcId` from NVIDIA CloudMatch again
  (`prod.cloudmatchbeta.nvidiagrid.net`), not the login provider's streaming URL.
  A partner or regional `serverId` still succeeds against `games.geforce.com` but
  returns a degenerate library of about four to seven titles — the same list on
  every account. Session create still uses the provider URL.
- UI language is written to `settings.json` and restored on launch. Changing it
  in the App tab previously only lasted until the process exited, so every
  relaunch was English.

## [0.3.1] - 2026-08-02

Error handling pass on top of 0.3.0: launch failures now speak GeForce NOW's
own error codes instead of raw JSON, and two causes of a stuck or locked-out
launch are fixed.

### Changed

- Failed launches now say what went wrong. GeForce NOW's error codes are modelled
  properly — 121 of them, 69 with wording in English and Spanish, ported from
  OpenNOW's `gfnErrorCodeEnum.ts` — so a failure shows "Membership Upgrade
  Required" or "Region At Capacity" instead of a truncated dump of NVIDIA's JSON.
  An unrecognised code is named rather than pasted.
- The code also drives behaviour, replacing three separate substring classifiers
  that had grown up around the error text — one picking the error screen's
  wording, one deciding whether to refresh the token, one deciding whether a
  catalog failure was an authorization problem. They matched on text that had
  *already been translated*, so a Spanish player took different branches than an
  English one.
- A poll failure NVIDIA reports as final now stops immediately instead of
  spending the whole 5xx retry allowance — about 2.5 minutes — re-asking about a
  banned region or a membership problem.
- The catalog now defaults to the account's owned games instead of the whole
  GFN catalog (same `library.status` filter as OpenNOW's `LIBRARY_APPS_FILTER`),
  with a "My Games / All Games" picker next to the sort dropdown to switch back.

### Fixed

- Launches abandoned with `HTTP 503` while NVIDIA was installing a game update.
  CloudMatch reports patching as a 5xx with `statusCode` 41, which counted
  against the server-error allowance and gave up after ~2.5 minutes — far short
  of how long a patch takes. It is now recognised as progress, with its own
  "Updating the game" screen. Mirrors OpenNOW-Switch's `IsAppPatchingResponse`.
- Launch failures now lead with the CloudMatch status code and description
  instead of opening with raw JSON that the error screen truncates.
- "A session is already open" locking an account out for the several minutes
  NVIDIA takes to reap a session, after a crash or a force-quit. Three causes:
  - The device id sent to CloudMatch was a UUIDv5 of a fixed string, so it was
    *identical on every Vita running the app* — and NVIDIA refuses a `DELETE`
    from a device that does not own the session. It is now the same per-install
    id sign-in already persisted, matching OpenNOW-Switch's `GenerateDeviceId`.
  - The open session is now written to the memory card with the zone that
    provisioned it, and deleted on the next launch. The normal stop does not run
    when the process dies, and asking CloudMatch which sessions are open does not
    say which zone to delete them at.
  - Both cleanup paths deleted against the generic CloudMatch entry point rather
    than the session's own zone, so they could not remove anything.

## [0.3.0] - 2026-07-31

Closes the two gaps 0.2.1 shipped with — no rear-touch mapping for the analog
triggers or L3/R3, and no way to type — and turns the stream from "it runs" into
something tunable.

### Added

#### Input
- In-game keyboard, on the Vita's inline IME. Characters, Backspace, Enter and
  the arrow keys are inferred from the IME's buffer edits and forwarded to the
  game as real keystrokes.
- Rear touch panel mapped to L2/R2, with selectable trigger intensity.
- L3/R3 zones on the front touch screen, optionally drawn over the stream.
- Trackpad mode: the front panel as a mouse, for games that want one.
- In-stream toolbar — exit, stats, control settings, trackpad and keyboard —
  collapsible so it stays out of the picture.

#### Catalog
- Favourites, kept on the memory card. Each entry stores enough of the game to
  draw its row, so a favourite past the catalog's 1000-title page cut-off still
  appears instead of vanishing until you search for it.
- Sorting by recently played, recommended, or title.

#### Streaming
- Opus pipeline reworked with a jitter buffer, RED packet recovery and a gain
  stage. NVST audio arrives out of order often enough to matter over 2.4 GHz,
  and much quieter than a local GameStream host.
- Audio boost, selectable and persisted.
- Link estimation: the client remembers what the network actually delivered and
  asks for a ceiling the link has been seen to reach, rather than a hardcoded
  guess that costs the opening seconds of every session in lost packets and
  resolution drops.
- Selectable frame rate, persisted between sessions.
- Stats overlay for the live session.

#### Platform
- CPU/GPU clocks raised to a streaming profile. The Vita boots homebrew at
  conservative clocks, and the shell loop paces the whole video pipeline.
- Explicit thread-to-core affinity across the three user cores, so the shell
  loop, video decode and network threads stop contending for the same one.

### Fixed

- In-game keyboard taking the firmware down with `C2-12828-1` the moment it
  opened. Four independent causes, found by diffing against vita-moonlight's
  `keyboardsystem.c`:
  - `SCE_SYSMODULE_IME` was never loaded, so the first call into libime jumped
    through an unresolved import.
  - `sdkVersion` was a hand-written guess instead of `PSP2_SDK_VERSION`.
  - The IME event handler called back into libime (`sceImeSetText` /
    `sceImeSetCaret`). The caret reset now runs on the owning thread in
    `update()`, behind a flag the handler raises.
  - `initialText` and `inputTextBuffer` pointed at the same buffer, leaving
    libime reading the text it was concurrently writing.
- `sceImeOpen` ran on a scratch thread that exited immediately, leaving
  `sceImeUpdate` pumping a session whose owning thread was gone. Every libime
  call now shares the shell loop's thread.
- SDL's text input — itself an IME dialog — is no longer started while the
  inline IME is open. libime does not tolerate both at once.
- Double `sceImeClose` when the keyboard was dismissed from the IME's own close
  button.

## [0.2.1] - 2026-07-28

First public release.

### Added

- On-console NVIDIA login with device-code flow and encrypted tokens.
- Game library with cover art and server-side search.
- Session brokering through CloudMatch, with queue tracking.
- WebRTC streaming: NVST signalling and H.264 depacketization.
- Hardware video decode via `sceAvcdec`.
- Opus audio decode and playback through SDL2.
- Controller input at 60×/s over the NVST data channel.
- Session resilience for transient failures.
- English and Spanish UI.

### Known gaps

- Analog triggers and L3/R3 had no rear-touchpad mapping. *(Addressed in 0.3.0.)*

[0.4.0]: https://github.com/zero-phoenix/OpenNOW-vita/releases/tag/v0.4.0
[0.3.1]: https://github.com/OpenCloudGaming/OpenNOW-vita/releases/tag/v0.3.1
[0.3.0]: https://github.com/OpenCloudGaming/OpenNOW-vita/releases/tag/v0.3.0
[0.2.1]: https://github.com/OpenCloudGaming/OpenNOW-vita/releases/tag/v0.2.1
