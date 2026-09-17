# Changelog

All notable changes to OpenNOW Vita are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] - 2026-09-17

Control rewrite, and the reason the last one could not be trusted: **the tests had never run.**
`src/streaming/audio.rs` and `src/shell/mod.rs` linked ARM `libopus` and a static SDL2
unconditionally, and the crate had no library target, so `cargo test` could not build at all on a
PC. Every previous "verified" claim rested on the code compiling and linking. This release fixes
that first, and the very first run of the previously-unrunnable tests failed - see below.

### Added

- **`opennow-core`**, a dependency-free workspace crate holding the NVST wire format and the whole
  input mapping. It builds and tests on any PC in seconds: **73 tests**, covering touch routing,
  the zone layout, every binding, the pointer and scroll maths, and the packet encodings.
- **One layout table.** `core/src/input/layout.rs` holds every on-screen zone once, and both the
  renderer and the hit-test read *that table*. The previous design had two functions "derived from
  the same constants", which is not the same thing and is how a button ends up drawn where it
  cannot be pressed. Three tests enforce it: no two live zones overlap, nothing can cover the eye,
  and every drawn zone answers at its own centre.
- **A written precedence order.** `route_touch` replaces a 25-line `if`/`else if` chain inside the
  60 Hz event loop with one function whose ordering is a list you can read.
- **A minimal key strip in the game profile**: `ESC`, `Enter`, the keyboard toggle and `Alt+F4`
  along the top edge, always available. The front screen is dead space in the game profile anyway -
  NVST carries no touch to the title - so this costs the pad nothing.
- **Tap-to-fire, not press-to-fire.** Strip keys fire when the finger *lifts*, and only after a
  short, still press. That is what makes a permanently visible strip safe while playing: a thumb
  resting on the top edge no longer fires Escape into the game.
- **Shift in chords.** `SendChord` now holds all four modifiers as real keys. `Ctrl+Shift+Esc` and
  the whole family of Shift shortcuts were previously impossible to send.
- **A Windows-shortcut page** on the on-screen keyboard: Start, Show desktop, Explorer, Task view,
  Maximise, Alt+Tab, Alt+F4, Task manager, Copy, Paste, Undo, Select all, Fullscreen, Lock, the
  security screen and Search - one tap each.
- **Missing keys**: `Insert`, `Delete`, `PageUp`, `PageDown` and `CapsLock`. Their constants already
  existed in the protocol; they had simply never been put on a keycap.
- **Auto-fade** for the game profile's strip: it dims to 35 % after six idle seconds and snaps back
  on contact, so "always visible" stops competing with the picture. Switchable in
  **Settings -> Controls**.
- **Input diagnostics** in the stats overlay (`l3 r3 l2 r2`), so a "the controls don't work" report
  can be checked against what the router decided rather than guessed at from source.
- **A reproducible local build** (`scripts/Dockerfile.build`, `scripts/docker-build.sh`): the same
  VitaSDK container CI uses, so a local build and a CI build are the same build.

### Changed

- **The front screen no longer drives the cursor in the desktop profile.** The trackpad branch was
  evaluated *before* the desktop-profile branch and the preference defaults to on, so the "the
  middle of the screen is deliberately dead space" branch below it was unreachable. The comment was
  right about the intent and wrong about the behaviour.
- **The control manual is off the picture.** It was painted across the centre of the screen in
  *both* profiles whenever the overlay was revealed - a text card over Death Stranding for the whole
  session. It now flashes for four seconds after a profile switch, which is the one moment it is
  wanted, and otherwise lives in Settings.
- **The manual is generated from the bindings table** rather than written out by hand, so it cannot
  describe a layout the client does not have.
- **L3/R3 corners shrank** from the bottom third of the screen (`y >= 0.66`) to `y >= 0.80`, handing
  about 87 px of picture back. They stay live when the overlay is hidden: they are pad buttons, not
  overlay controls.
- **The pad is sampled at 120 Hz on its own clock**, not once per rendered frame. Sampling inside
  the render loop meant a 30 ms frame also delayed the controller by 30 ms - a render hiccup and
  input lag were the same event. They are now independent.
- **Reading a preference no longer clones the whole settings struct.** `pc_overlay_enabled()`,
  `control_profile()` and `overlay_revealed()` were called once per SDL event inside the event loop,
  and each returned a `clone()` of an `AppSettings` containing a `BTreeMap` and eight `String`s.
  With a thumb dragging across the panel that is dozens of clones and hundreds of allocations per
  frame - not much CPU, but a steady supply of the heap fragmentation that surfaces as a stall with
  no obvious cause. All twenty accessors now read through the cache, and the mapping is handed one
  `Copy` snapshot per frame instead of reaching into global state.

### Fixed

- **The client crashed on startup on the current toolchain, before drawing a frame.** SDL's
  renderer was asked for "the first accelerated driver", and as of the September 2026 VitaSDK image
  that is no longer GXM: SDL2 is now built with the GLES2 renderer on top of vitaGL, registered
  ahead of `VITA gxm`. Trying GLES2 recreates the window with `SDL_WINDOW_OPENGL`, which runs
  `vglInitExtended` and takes ownership of GXM; GLES2 then fails, because vitaGL needs
  `libshacccg.suprx` to compile its shaders and neither Vita3K nor a stock Vita has that module;
  SDL falls back to `VITA gxm`, whose `sceGxmCreateContext` now answers
  `SCE_GXM_ERROR_ALREADY_INITIALIZED` - and the renderer dereferences the context it did not get.
  The renderer is now pinned to `VITA gxm` by index, and logs which driver it actually got. Going
  through GL would have been the wrong trade regardless: the direct-texture path exists to hand
  BGR565 frames straight to GXM.
- **A latent bug in `encode_gamepad_state_partially_reliable`**, found by running its own test for
  the first time: it asserts a 54-byte frame, its doc comment describes a 42-byte one, and the code
  emits 40, because the inner payload is 24 bytes where its own length field claims 26. The encoder
  is unused, so nothing was broken - but three different numbers in one function is exactly what a
  "compiles and links" check cannot see. Left as-is rather than guessed at (inventing two bytes of a
  binary protocol is worse than no partial reliability), with the test now pinning what it really
  emits and a comment explaining what to check it against.
- **Modifiers can no longer be stranded on the host.** A `ModifierLatch` tracks what was actually
  pressed - as distinct from what is merely *latched* - and releases exactly that on profile switch
  and session end. A property test over 500 random sequences asserts every press is eventually
  released and nothing is ever released twice.
- **CRLF line endings no longer break the build on Windows.** A checkout with `core.autocrlf` on
  rewrote the VitaSDK wrapper scripts, turning their shebang into `/bin/sh\r`, and the kernel then
  reports "No such file or directory" for a file that is plainly there. `.gitattributes` pins them.

### Build

- **The VPK links again.** SDL2's installed `pkg-config` data lists neither `SceShaccCgExt` nor
  `stdc++`, so as of the September 2026 image nothing pulled in what libvitaGL and libvitashark
  now need, and the link step died in a wall of `undefined reference to std::__throw_length_error`
  and `sceShaccCgExtEnableExtensions`. Untouched `master` failed identically on the same image, so
  this is the toolchain having moved, not a regression here. Four libraries, in the order ld needs
  them: `-lSceShaccCgExt -lSceShaccCg_stub -lstdc++ -ltaihen_stub_weak`. The taiHEN stub is the
  *weak* one deliberately - a normal taiHEN import makes module load fail wherever taiHEN is
  absent, which is exactly the case under Vita3K.
- **A reproducible local build**, in the same container CI uses: `scripts/Dockerfile.build` plus
  `scripts/docker-build.sh`, which runs the host tests and then delegates to the `Makefile` so the
  link flags have exactly one definition. Mounting a named volume at `/root/.cargo/registry` turns
  a 16-minute cold build into a 3-minute one.

## [0.5.0] - 2026-09-13

Complete redesign of the on-screen controls. 0.4.x shipped the PC-touch overlay switched **off**
by default and, when it was switched on, it quietly replaced the gamepad snapshot with a neutral
one — so the overlay was invisible unless you went looking for it, and turning it on broke L2/R2,
L3/R3 and half the D-pad. Both of those are gone.

### Added

- **Two control profiles, switchable mid-stream.** The Vita has no physical L2/R2: they only
  exist as rear-panel zones, so "rear panel = mouse" and "rear panel = triggers" genuinely cannot
  coexist. Rather than half-doing both, the client now has two explicit profiles and swaps
  between them instantly:
  - **Game** — every stick, button, trigger zone and stick-click reaches the title untouched.
    This is what you want for Death Stranding, Silent Hill f and anything else made for a pad.
  - **Desktop** — the Vita becomes a mouse and keyboard for the Windows session that
    Install-to-Play titles boot into.
- **Always-visible eye toggle** (top-right of the front screen, deliberately drawn at a higher
  alpha floor than everything else so it can never become invisible). Tapping it reveals or
  hides the whole overlay; it is live in both profiles, so you can never lock yourself out.
- **Profile switch** directly under the eye, live in both profiles while revealed — the only
  touch route out of the game profile, where the key strips are intentionally dead.
- **Minimalist control manual**, drawn in the middle of the screen while the overlay is
  revealed: one line per stick/button with what it does in the *active* profile, so you never
  have to leave the stream to remember the layout.
- **Rear panel is now the mouse** in the desktop profile, including clicks — left on the left
  half, right on the right half. Earlier builds refused to click from the rear panel on the
  grounds that it is out of sight; in practice that left no way to click at all without covering
  the picture with a thumb. A click only fires when the finger lifts within 300 ms *and* moved
  less than 5 % of the panel, so a cursor drag never clicks by accident.
- **Analog sticks in the desktop profile**: left stick is a fine cursor (with sub-pixel
  accumulation, so slow nudges are not rounded away), right stick is the scroll wheel in whole
  ±120 notches, D-pad types the arrow keys with hold-to-repeat, Cross/Circle are held left/right
  mouse buttons, Triangle/Square are Enter/Backspace, L is precision mode (halves sensitivity),
  R is a double-click, SELECT toggles the keyboard and START taps the Windows key.

### Changed

- **The overlay is on and revealed by default.** It used to default to off, which is why the
  previous release looked unchanged on real hardware.
- **Redesigned front-screen layout.** The old design put zones on all four edges *and* both
  bottom corners, bracketing the picture and stealing the corners `FrontStickZones` needs. Zones
  now live in two thin strips along the top and bottom plus two narrow slider rails, leaving the
  entire middle of the 960×544 panel clear. Top strip: ESC, Tab, Win, Alt+Tab, Copy, Paste,
  keyboard, settings. Bottom strip: Shift, Ctrl, Alt, Enter, Backspace, Ctrl+Alt+Del. Left rail
  adjusts mouse sensitivity, right rail scrolls.
- **The overlay no longer neutralizes the gamepad.** Touch ownership is arbitrated by
  precedence on finger-down instead: the overlay claims a touch only when its zones are actually
  live, and the stick zones take anything it did not claim. In the game profile L3/R3 behave
  exactly as they do with the overlay off.
- **Opacity now has five steps** (Ghost / Faint / Normal / Strong / Bold) instead of three, and
  legacy stored values snap to the nearest new preset rather than resetting.
- **The on-screen keyboard is translucent**, tied to the same opacity preference, so you can see
  what you are typing into.
- **Raised the adaptive bitrate ceiling** from 12 to 20 Mbps (and a first-run session now asks
  for 12 instead of 8). 960×544 is a small frame, but it is the panel's *native* resolution, so
  every encoder artefact lands on a real pixel with no downscale to hide it, and 12 Mbps left
  visible mush in dark, high-motion scenes. This only raises the ceiling the measured estimate
  may climb to; the lowering path is unchanged, so a weak link still ratchets straight back down.

### Fixed

- Turning the PC overlay on no longer kills L2/R2, L3/R3 and D-pad up/down.
- The centre of the front screen is no longer contested: in the desktop profile it is deliberate
  dead space (the rear panel is the pointer), and in the game profile it reaches the title.

## [0.4.1] - 2026-09-13

### Added

- **"Force direct NVIDIA login" setting** (Settings → Account, on by default): the device-code
  login flow used to always call `providers::discover_providers()`, an unauthenticated request
  to `pcs.geforcenow.com/v1/serviceUrls` that returns a login provider chosen by NVIDIA's
  backend from server-side network/ISP signals — not from the client's actual location or VPN
  exit node. For accounts whose traffic gets classified that way, this could silently hand the
  login to a regional whitelabel partner (e.g. "GeForce NOW powered by Digevo", NVIDIA's
  official Peru-market reseller) instead of NVIDIA's own login, even while tunnelling through a
  US VPN — surprising for a US-registered Ultimate + persistent-storage account with no
  intention of using a regional partner. With the new preference on, the client skips
  `discover_providers()` entirely and always authenticates against NVIDIA's own default idp and
  streaming endpoint (`GfnProvider::default()`), so the sign-in screen no longer depends on
  what the discovery endpoint feels like returning that day.

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

[0.4.1]: https://github.com/zero-phoenix/OpenNOW-vita/releases/tag/v0.4.1
[0.4.0]: https://github.com/zero-phoenix/OpenNOW-vita/releases/tag/v0.4.0
[0.3.1]: https://github.com/OpenCloudGaming/OpenNOW-vita/releases/tag/v0.3.1
[0.3.0]: https://github.com/OpenCloudGaming/OpenNOW-vita/releases/tag/v0.3.0
[0.2.1]: https://github.com/OpenCloudGaming/OpenNOW-vita/releases/tag/v0.2.1
