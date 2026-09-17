# OpenNOW Vita

[![Build VPK](https://github.com/zero-phoenix/OpenNOW-vita/actions/workflows/build.yml/badge.svg)](https://github.com/zero-phoenix/OpenNOW-vita/actions/workflows/build.yml)
[![Latest release](https://img.shields.io/github/v/release/zero-phoenix/OpenNOW-vita?label=latest%20release)](https://github.com/zero-phoenix/OpenNOW-vita/releases/latest)
[![License: MPL-2.0](https://img.shields.io/badge/license-MPL--2.0-blue.svg)](LICENSE)

**OpenNOW Vita** is a native homebrew **GeForce NOW client for the PlayStation Vita**, written
entirely in Rust. It signs in to your GFN account on the console itself, lists your game
library with cover art, negotiates a real WebRTC session against NVIDIA's streaming
infrastructure, and decodes the incoming H.264 video on the Vita's own hardware decoder — with
your controller, and optionally a full keyboard+mouse, forwarded back into the stream in real
time. No PC, phone, or browser is involved once you're signed in.

This fork of [OpenCloudGaming/OpenNOW-vita](https://github.com/OpenCloudGaming/OpenNOW-vita)
adds **two switchable control profiles** - one that forwards every control to the game, and one
that turns the Vita's touchscreens, sticks and buttons into a keyboard+mouse for the remote
desktop — handy for driving Windows, Steam, or Epic through a
GeForce NOW Ultimate **Install-to-Play** session — plus a set of reliability fixes around
Install-to-Play catalog visibility, session cleanup, and sign-in routing. See
[What this fork changes](#what-this-fork-changes) for the full list.

Architecturally it follows the trail blazed by
[green-vita](https://github.com/Day-OS/green-vita) (Xbox Cloud Gaming on the Vita): SDL2 +
egui for the UI, direct-to-texture hardware video decoding, and VPK packaging via
`cargo-vita`. The GFN protocol work builds on
[OpenNOW](https://github.com/OpenCloudGaming/OpenNOW) and OpenNOW-Switch as references.

> **Disclaimer**: this project is **not affiliated with, endorsed by, or associated with
> NVIDIA or GeForce NOW** in any way. It is an unofficial alternative client. You need your
> own GeForce NOW account to use it.

## Table of contents

- [Features](#features)
- [What this fork changes](#what-this-fork-changes)
- [Control profiles: playing games vs driving Windows](#control-profiles-playing-games-vs-driving-windows)
- [Sign-in routing: avoiding regional partner logins](#sign-in-routing-avoiding-regional-partner-logins)
- [Status](#status)
- [Getting a build](#getting-a-build)
- [Build requirements](#build-requirements)
- [Building locally](#building-locally)
- [Tests](#tests)
- [Continuous integration & releases](#continuous-integration--releases)
- [Project layout](#project-layout)
- [Acknowledgements](#acknowledgements)

## Features

- **NVIDIA login on the console** — device-code flow (QR code + short code on a second
  device), with tokens encrypted at rest (ChaCha20-Poly1305, key stored in the Vita's Safe
  Memory). A "force direct NVIDIA login" setting (on by default) skips server-side login
  provider discovery so the client always signs in against NVIDIA directly instead of
  whatever regional partner that discovery call returns — see
  [below](#sign-in-routing-avoiding-regional-partner-logins).
- **Game library** — your GFN catalog with cover art, server-side search, an "owned games"
  filter with a My Games/All Games picker, sorting (recently played, recommended, title), and
  favourites that are kept on the memory card so they survive past the catalog's page cutoff.
- **Install-to-Play aware** — variant selection prefers the variant you actually own
  (`gfn.library.selected`/owned status) over blindly picking the first numeric-id catalog
  variant, so a title whose Install-to-Play/Steam variant has a non-numeric id — which used to
  make it disappear or launch the wrong variant — appears and launches correctly, and
  `accountLinked` in the session request reflects real per-game ownership.
- **Session brokering** — CloudMatch session creation, queue position tracking, seat-aware
  polling (re-polls the assigned game server directly once seated), and pre-launch cleanup of
  zombie sessions across every zone the client currently knows about — not just the pinned
  one — so a stale session on a different zone can't lock you out with a false "device limit
  reached".
- **Real WebRTC streaming** — NVST WebSocket signaling, SDP offer/answer against NVIDIA's
  ICE-lite servers, DTLS-SRTP, and H.264 RTP depacketization, all through the sans-I/O
  [`rtc`](https://github.com/webrtc-rs/rtc) stack (no GStreamer, no browser).
- **Hardware video decoding** — `sceAvcdec` decodes each access unit straight into SDL/GXM
  textures (green-vita's direct-texture path: zero per-frame allocations, double-buffered,
  dynamic YUV420/BGR565 negotiation based on the stream's actual resolution), with a
  video-stall watchdog and selectable frame rate.
- **Audio playback** — Opus RTP packets decoded via `libopus` and played through SDL2, with a
  jitter buffer, RED packet recovery, a selectable gain/boost stage, and small buffers on both
  audio and video so neither track drifts ahead of the other.
- **Controller input** — full gamepad state (buttons, sticks) sent 60×/s over the NVST
  `input_channel_v1` data channel in XInput format.
- **Rear touch panel as analog triggers** — L2/R2 mapped to the back touchpad (the Vita has no
  physical analog triggers), with selectable trigger intensity, plus L3/R3 zones on the front
  screen. In the desktop control profile the rear panel becomes the mouse instead — see below.
- **In-game keyboard** — the Vita's inline IME, wired so character/Backspace/Enter/arrow edits
  are forwarded to the stream as real keystrokes.
- **Two control profiles** — forward everything to the game, or turn the Vita into a mouse and
  keyboard for the remote Windows desktop; see
  [below](#control-profiles-playing-games-vs-driving-windows).
- **Region pinning** — `src/gfn/regions.rs` lets the client pin a specific streaming
  zone/region instead of always taking NVIDIA's default geo-routed one, useful where the
  automatically-selected zone isn't the best link for your ISP.
- **Session resilience** — CloudMatch session polling tolerates transient server errors
  (isolated 5xx responses from NVIDIA's zone load balancer) instead of aborting a session
  that would have come up fine on the next poll; disconnects clean up the CloudMatch session
  server-side instead of leaking it.
- **Link estimation** — the client remembers what the network actually delivered in past
  sessions and asks for a bitrate ceiling the link has been seen to reach, instead of a
  hardcoded guess that costs the opening seconds of every session to lost packets and
  resolution drops.
- **In-stream toolbar** — exit, stats (kbps/loss/RTT), control settings, trackpad and keyboard
  toggles, collapsible so it stays out of the picture.
- **Platform tuning** — CPU/GPU clocks raised to a streaming profile, explicit thread-to-core
  affinity across the Vita's three user cores so the shell loop, video decode, and networking
  threads stop contending for the same one.
- **Language picker** — a gear icon next to the account avatar switches the UI between
  English, Spanish, French, and Russian (more languages can be added under `src/i18n/`); the
  choice is persisted and restored on relaunch.

## What this fork changes

Relative to upstream [OpenCloudGaming/OpenNOW-vita](https://github.com/OpenCloudGaming/OpenNOW-vita),
this fork adds:

1. **[Two control profiles](#control-profiles-playing-games-vs-driving-windows)** — a *game*
   profile that forwards every stick, button and trigger to the title untouched, and a
   *desktop* profile that turns the Vita into a mouse and keyboard for the Windows session
   Install-to-Play titles boot into, with an always-visible eye toggle to swap between them
   mid-stream and an on-screen manual for each.
2. **Install-to-Play catalog/launch fixes** — titles you own on a linked store (Steam, Epic)
   whose Install-to-Play variant has a non-numeric id no longer vanish from the library or
   launch under the wrong variant; `accountLinked` reflects real ownership instead of a
   hardcoded value.
3. **Cross-zone zombie-session cleanup** — a session left open on a zone other than the
   currently-pinned one (e.g. after a crash, or after playing the same game from a PC) no
   longer produces a false "device limit reached" on the next launch attempt.
4. **[Direct NVIDIA login routing](#sign-in-routing-avoiding-regional-partner-logins)** — an
   opt-out from server-side login-provider discovery, so sign-in doesn't get silently handed
   to a regional whitelabel partner.
5. **GitHub Actions CI** — every push and pull request builds a real Vita `.vpk` inside the
   official VitaSDK container, and every `v*` tag publishes it as a GitHub Release with
   release notes pulled straight from `CHANGELOG.md` — no more manual builds to hand someone
   a working `.vpk`.
6. **[Tests that actually run](#tests)** — the input mapping and the NVST wire format live in
   `opennow-core`, a dependency-free crate that builds on any PC. Before 0.6.0 `cargo test`
   could not build at all: the binary linked ARM `libopus` and a static SDL2 unconditionally
   and there was no library target, so "verified" only ever meant "it compiled". 73 tests now
   run in under a second, and the first run of them found a real bug.

See `CHANGELOG.md` for the complete, dated history of every change, including everything
inherited from upstream.

## Control profiles: playing games vs driving Windows

A GeForce NOW **Install-to-Play** session does not drop you into a game — it drops you into a
**Windows 11 desktop**, where you have to click through Steam or the Epic launcher before the
game ever starts. A gamepad is useless there, and a mouse is useless once the game starts. The
Vita has to be both, and it has to switch between them without ending the session.

It cannot be both *at once*. The Vita has no physical L2/R2 triggers — they exist only as zones
on the rear touch panel — so "rear panel is the mouse" and "rear panel is L2/R2" are genuinely
mutually exclusive. Rather than half-doing both and leaving you with an unreliable version of
each, this fork has **two explicit control profiles** and makes swapping between them a single
tap:

| | **Game profile** (default) | **Desktop profile** |
|---|---|---|
| Sticks | to the title | left = fine cursor, right = scroll wheel |
| D-Pad | to the title | arrow keys, with hold-to-repeat |
| Face buttons | to the title | ✕/○ = left/right mouse button (held), △ = Enter, □ = Backspace |
| L / R | L1 / R1 | L = precision mode (½ sensitivity), R = double-click |
| Rear panel | L2 / R2, pressure-graded | mouse cursor + click (left half / right half) |
| Front bottom corners | L3 / R3 | modifier & key strip |
| Front top edge | `ESC` `⏎` `⌨` `ALT+F4` | full key strip |
| SELECT / START | to the title | on-screen keyboard / Windows key |

The game profile forwards **everything** to the title, untouched — it is byte-for-byte what you
get with the overlay switched off, which is what makes Death Stranding- and Silent Hill f-class
titles actually playable while the overlay is still on screen. A test enforces it: if a future
edit ever repurposes a control there, `the_game_profile_forwards_every_control` fails before the
change reaches a console.

The four keys along the top edge are the one exception, and they cost the pad nothing: NVST
carries no touch to the title, so in the game profile the front screen is dead space anyway.
They fire when your finger **lifts**, and only after a short, still press — a thumb resting on
the top edge while you play does not fire Escape into the game.

### The eye

There is always a small semi-transparent **eye** in the top-right corner of the front screen. It
is drawn at a higher minimum opacity than everything else, on purpose: it is the way back, so it
must never be able to fade into the picture.

- **Tap the eye** — reveal or hide the entire overlay.
- **Tap the switch directly under the eye** — swap between the game and desktop profiles. It is
  live in both profiles, so you can never strand yourself in game mode with no way out.
- After a **profile switch**, a minimalist control manual flashes in the middle of the screen for
  four seconds, listing what every stick and button does in the profile you just switched *into*.
  It is generated from the bindings table, so it cannot describe a layout the client does not
  have, and it does not linger: 0.5.0 drew it permanently, in *both* profiles, which put a text
  card over the picture for the whole session.

In the game profile the key strip **dims to 35 % after six idle seconds** and snaps back the
moment you touch the screen — always visible, without competing with the picture. Overlay
on/off, revealed/hidden, the active profile, opacity (five steps, Ghost → Bold), the idle
dimming and mouse sensitivity are all persistent preferences under **Settings → Controls**.

### Front-screen layout (desktop profile)

The keys live in two thin strips along the top and bottom edges plus two narrow slider rails,
which leaves the **entire middle of the 960×544 panel clear**. The earlier 0.4.x design put
zones on all four edges *and* both bottom corners; it bracketed the picture and stole the
corners the stick zones need.

```
┌──────────────────────────────────────────────────────┬────┐
│ ESC  TAB  ⊞  ALT⇥  COPY  PASTE  ⌨  ⚙                 │ 👁 │
├──────────────────────────────────────────────────────┼────┤
│                                                      │ 🎮 │
│ ▲                                                  ▲ └────┘
│ │ DPI                                        SCROLL │      
│ ▼                                                  ▼       
│                                                            │
├────────────────────────────────────────────────────────────┤
│ SHIFT  CTRL  ALT  ⏎  ⌫  SUPR  ATAJOS  C-A-DEL              │
└────────────────────────────────────────────────────────────┘
```

Shift/Ctrl/Alt are **sticky** modifiers, shared with the on-screen keyboard's own modifier
state, so `Ctrl` then a letter from the keyboard is a real chord — and all four modifiers,
Shift included, are now sent as real held keys, which is what makes `Ctrl+Shift+Esc` reach the
host at all. `ATAJOS` opens a page of one-tap Windows shortcuts (Start, show desktop, Explorer,
task view, Alt+Tab, Alt+F4, task manager, lock, and the rest).

Every zone above comes from one `const` table in `core/src/input/layout.rs`, which the renderer
and the hit-test both read. That is deliberate: they used to be two functions "derived from the
same constants", which is not the same thing and is how a button ends up drawn somewhere it
cannot be pressed. Three tests hold the line — no two live zones overlap, nothing can ever cover
the eye, and every drawn zone answers at its own centre.

### Rear panel as a mouse

The NVST input protocol has only a *relative* mouse-move packet (`INPUT_MOUSE_MOVE_REL`, clamped
at ±4096 per axis) — there is no absolute-position packet that could move the host cursor to an
exact point. The rear panel is therefore mapped as a **relative trackpad**: dragging moves the
cursor by a delta, like a laptop trackpad, not a 1:1 touch-to-pixel map. A configurable
sensitivity multiplier scales that delta, and holding **L** halves it for precision work.

It also **clicks**: a tap on the left half is a left click, the right half a right click.
Earlier builds deliberately refused to click from the rear panel, on the grounds that the panel
is out of sight and a stray tap would be hard to notice. In practice the opposite was true —
with no rear click there was no way to click at all without covering the picture with a thumb.
The safeguard is that a click only fires when the finger lifts within **300 ms** *and* travelled
less than **5 %** of the panel; anything slower or further is a cursor drag and clicks nothing.

### Scroll wheel

`INPUT_MOUSE_WHEEL` was added to `src/gfn/input_protocol.rs`, ported from the OpenNOW /
OpenNOW-Switch reference clients' wire format rather than invented, and emitted in whole ±120
notches (the `WHEEL_DELTA` convention Windows expects). Both the right stick and the right-hand
slider rail drive it.

### Picture quality

The stream is already requested at the panel's **native 960×544** with linear filtering and
32-bit colour, so there was no resolution left to gain. What there was to gain was bitrate: the
adaptive ceiling was capped at 12 Mbps, and at native resolution every encoder artefact lands on
a real pixel with no downscale to hide it, which showed as mush in dark, high-motion scenes.
0.5.0 raises the ceiling to **20 Mbps** (first-run estimate 8 → 12). This only changes the
maximum the measured estimate may climb to — the lowering path is untouched, so a weak link
still ratchets straight back down.

Relevant source: `src/input.rs` (touch-zone geometry, profile-aware routing, stick/button
mapping), `src/shell/mod.rs` (touch-ownership arbitration), `src/gfn/input_protocol.rs` (mouse
move/wheel packet encoding), `src/gfn/stream_prefs.rs` (persisted prefs), `src/app/ui.rs`
(egui overlay rendering and the control manual), `src/gfn/link_estimate.rs` (bitrate ceiling).
## Sign-in routing: avoiding regional partner logins

GeForce NOW's device-code sign-in normally starts by asking NVIDIA's backend which login
provider to use, via an unauthenticated `pcs.geforcenow.com/v1/serviceUrls` lookup. That
lookup is server-side and can be influenced by network/ISP-level signals rather than by
where you actually are or which VPN exit node you're using — which means it can hand your
sign-in to a **regional whitelabel partner** (for example, NVIDIA's own Peru-market
"GeForce NOW powered by Digevo" branding) instead of NVIDIA's direct login, even for an
account that has nothing to do with that region.

Settings → Account has a **"Force direct NVIDIA login"** toggle, on by default, that skips
that discovery call entirely and always signs in against NVIDIA's own default identity
provider and streaming endpoint (`GfnProvider::default()` in `src/gfn/providers.rs`). Turn it
off only if you specifically want the provider your account/network would normally be routed
to.

## Status

| Phase | Scope | State |
|---|---|---|
| 0 | Protocol research (`docs/protocol-notes.md`) | ✅ Done |
| 1 | App skeleton: VitaSDK/`cargo-vita` build, SDL2 + egui loop | ✅ Done |
| 2 | Authentication + game library | ✅ Done |
| 3 | Signaling + CloudMatch session lifecycle | ✅ Done |
| 4 | WebRTC peer, H.264 decode, gamepad input | ✅ Working (Vita3K + real PS Vita hardware) |
| 5 | Audio (Opus), session resilience, UI polish | ✅ Working (Vita3K + real PS Vita hardware) |
| 6 | Real-hardware validation | ✅ Confirmed working on an original PS Vita |
| 7 | PC-Touch Overlay Mod (KB+mouse over the stream) | ✅ Implemented; smoke-tested boot/render in Vita3K |
| 8 | Direct NVIDIA login routing (skip partner discovery) | ✅ Implemented; each CI build re-verified booting in Vita3K |
| 9 | Control-profile redesign (game vs desktop, eye toggle) | ✅ Implemented in 0.5.0 |

Development is validated against both [Vita3K](https://vita3k.org/) (whose `sceAvcdec` only
implements YUV420 output, handled at runtime, and whose `sceNet` stack is stubbed — enough to
boot and render the UI, but not to complete NVIDIA's OAuth device-code login over a real TCP
connection) and real PS Vita hardware, which is required to validate networked login and
streaming end-to-end.

See `THIRD_PARTY_NOTICES.md` for what is reused from green-vita (MPL-2.0) and what is protocol
knowledge referenced from OpenNOW, and `CHANGELOG.md` for the full version history.

## Getting a build

You don't need to compile anything yourself: every push to `master` and every version tag
builds a fresh `.vpk` on GitHub Actions.

- **Latest tagged release (recommended)**: grab `opennow-vita.vpk` from the
  [Releases page](https://github.com/zero-phoenix/OpenNOW-vita/releases/latest) — each release
  is built and attached automatically by CI, with its description pulled straight from
  `CHANGELOG.md`.
- **Latest `master` build**: open the most recent successful run of the
  [Build VPK workflow](https://github.com/zero-phoenix/OpenNOW-vita/actions/workflows/build.yml)
  and download the `opennow-vita-vpk` artifact. This tracks unreleased work-in-progress and may
  be less stable than a tagged release.

Install the `.vpk` the normal homebrew way: copy it to the Vita (or `ux0:/data/`) and install
it from [VitaShell](https://github.com/TheOfficialFloW/VitaShell), or drop it straight into
[Vita3K](https://vita3k.org/).

## Build requirements

Only needed if you want to build locally instead of using a CI-produced `.vpk`.

- [VitaSDK](https://vitasdk.org/) installed, with the `VITASDK` environment variable
  pointing at it (this project does not install VitaSDK for you). The
  [`vitasdk/vitasdk`](https://hub.docker.com/r/vitasdk/vitasdk) Docker image is the fastest way
  to get a known-good toolchain without touching your host system — it's what
  `.github/workflows/build.yml` uses.
- Rust nightly + [`cargo-vita`](https://github.com/vita-rust/cargo-vita):
  ```sh
  rustup toolchain install nightly
  rustup component add rust-src --toolchain nightly
  cargo +nightly install cargo-vita
  ```
- `pkg-config` (e.g. `brew install pkg-config` on macOS, or already present in the
  `vitasdk/vitasdk` container).

## Building locally

```sh
make vpk                                    # builds target/armv7-sony-vita-newlibeabihf/release/opennow-vita.vpk
make upload-vpk VITA_IP=192.168.0.103       # uploads the VPK to ux0:/data/ via VitaShell/vitacompanion
make update-run-vita VITA_IP=192.168.0.103  # build + update + launch in one step
```

Uploading requires [VitaShell](https://github.com/TheOfficialFloW/VitaShell)'s FTP server or
`vitacompanion` running on the console, on the same network as your computer. The VPK also
installs and runs in the Vita3K emulator (subject to the `sceNet` stub limitation noted in
[Status](#status) above).

If you don't have VitaSDK installed natively, build in the container instead. `scripts/` has an
image definition matching what CI installs, so a local build and a CI build are the same build:

```sh
docker build -t opennow-build - < scripts/Dockerfile.build
docker run --rm -v "$PWD:/work" -w /work opennow-build sh scripts/docker-build.sh vpk
```

Keeping the toolchain in an image rather than reinstalling Rust and `cargo-vita` on every run
turns a rebuild from several minutes into seconds, which matters when you are chasing a compile
error rather than producing a release.

> **On Windows**: `.gitattributes` pins the `tools/vita-*` wrappers to LF. Without it a checkout
> with `core.autocrlf` on rewrites their shebang as `/bin/sh\r`, and the linker then reports
> *"No such file or directory"* for a file that is plainly sitting there. `docker-build.sh`
> repairs an existing checkout as well.

## Tests

```sh
cargo test -p opennow-core --target x86_64-unknown-linux-gnu
```

73 tests, under a second, on any PC — no VitaSDK, no console, no emulator. That is the whole
reason `opennow-core` exists as a separate crate: the binary links a static SDL2, ARM `libopus`
and the VitaSDK stubs unconditionally, so before 0.6.0 `cargo test` could not build at all and
"verified" could only ever mean "it compiled and linked".

What they cover, and why each one is there:

| Test | The bug it exists to prevent |
|---|---|
| `the_front_screen_does_not_drive_the_cursor_in_the_desktop_profile` | The reported one: the rear panel was the pointer, and the front screen was *also* the pointer |
| `no_two_live_zones_overlap_in_the_same_profile` | Two controls fighting over the same pixel, winner decided by iteration order |
| `nothing_can_cover_the_eye` | Losing the only way back out of a hidden overlay |
| `every_drawn_zone_answers_at_its_own_centre` | A button drawn where it cannot be pressed |
| `the_game_profile_forwards_every_control` | 0.4.x, where switching the overlay on silently killed L2/R2, L3/R3 and half the D-pad |
| `every_press_is_eventually_released` | A host left holding Ctrl after the session ends — a property test over 500 random sequences |
| `a_slow_stick_nudge_still_moves_the_cursor` | Sub-pixel movement rounding to zero every frame, so a gentle stick never moves at all |
| `the_middle_of_the_screen_belongs_to_nobody` | The overlay creeping back over the picture |

The Vita build itself is checked the only way it can be — by building it, locally in the
container above and in CI on every push.

## Continuous integration & releases

`.github/workflows/build.yml` defines two jobs:

- **`build`** — runs on every push/PR to `master` and on every `v*` tag, inside the
  `vitasdk/vitasdk` container. It installs Rust nightly + `cargo-vita`, runs `make vpk`, and
  uploads the resulting `.vpk` as a workflow artifact (`opennow-vita-vpk`).
- **`release`** — runs only when the trigger is a `v*` tag, after `build` succeeds. It
  downloads that artifact, extracts the matching version's section from `CHANGELOG.md` as the
  release description, and publishes a GitHub Release with the `.vpk` attached.

To cut a new release: bump `version` in `Cargo.toml` (and run `./scripts/sync-vita-version.sh`
to keep the Vita bubble's `APP_VER` in sync), add a dated section for it at the top of
`CHANGELOG.md`, commit, then push a matching tag:

```sh
git tag v0.4.1
git push origin v0.4.1
```

The workflow picks up the tag, builds, and publishes the release automatically — no manual
artifact upload needed.

## Project layout

```
.github/workflows/      CI: build the VPK on every push/PR/tag, publish tagged releases
.cargo/config.toml       Cross-compilation target/toolchain (armv7-sony-vita-newlibeabihf)
tools/                   vita-gcc/vita-ar/vita-pkg-config wrappers (VitaSDK)
static/sce_sys/          App metadata (icon, LiveArea) packaged into the VPK
scripts/sync-vita-version.sh   Keeps the VPK's APP_VER lined up with [package].version
scripts/Dockerfile.build       Build image matching CI, for reproducible local builds
scripts/docker-build.sh        Host tests + Vita build, run inside that image
core/                    opennow-core: the pure half, testable on any PC (73 tests)
  src/protocol.rs         NVST wire format: gamepad/key/mouse/wheel packet encoding
  src/config.rs           The settings the input mapping depends on, as one Copy snapshot
  src/input/layout.rs     THE layout table — renderer and hit-test both read this one list
  src/input/router.rs     route_touch(): whose finger is this, in a written precedence order
  src/input/bindings.rs   What each control means, per profile; the manual is generated from it
  src/input/mapper.rs     Sticks/buttons/touch -> events; ModifierLatch's press/release invariant
  src/input/physical.rs   The hardware state as plain data, plus the deadzone maths
src/
  main.rs                Entry point; Vita heap/stack sizing, CDRAM reservation
  logger.rs               File-backed logging (`ux0:/data/opennow/logs` on-device)
  app/
    mod.rs                Application state machine
    ui.rs                 egui UI: catalog, in-stream toolbar, PC-Touch overlay rendering
    settings_menu.rs       Settings tabs/rows (Stream, Controls, App, Account)
    fonts.rs               Bundled UI fonts
  shell/
    mod.rs                Main loop: SDL2 window/event pump, frame pacing
    egui_painter.rs        egui render backend on top of the Vita's GXM/SDL2 surface
    surface.rs             Direct video surface shared with the decode worker
  input.rs               Menu-side SDL2 event mapping and the AppCommand enum
  input_stream.rs        Layer 1 of the input stack: SDL events in, opennow-core values out.
                          Every decision it makes comes from `core/` and is covered by a test
                          there; what is left here is the part that genuinely cannot be
  jobs.rs                Background async task plumbing
  power.rs               CPU/GPU clock profile for streaming
  safe_memory.rs          Encrypted token storage in the Vita's Safe Memory
  thread_affinity.rs      Thread-to-core pinning across the Vita's user cores
  locale.rs               Supported UI locales (English, Spanish, French, Russian)
  i18n.rs, i18n/*.ftl     Fluent-based UI translations
  streaming/
    mod.rs                Streaming session plumbing shared by audio/video
    video/                Direct-texture video pipeline: decoder sync, sceAvcdec, decode worker
    audio.rs              Opus RTP decode, jitter buffer, RED recovery, SDL2 playback
  gfn/
    auth.rs               NVIDIA device-code OAuth + encrypted token storage; honours the
                          direct-login preference from `providers.rs`
    catalog.rs             Game library (GraphQL), search, Install-to-Play variant selection
    covers.rs              Cover-art cache with bounded async downloads
    favorites.rs            Memory-card-persisted favourites
    cloudmatch.rs           Session create/poll/stop against the CloudMatch REST API,
                            cross-zone zombie-session cleanup
    active_session.rs       Tracking of the locally-known open session for cleanup on relaunch
    signaling.rs            NVST WebSocket signaling (offer/answer/ICE trickle)
    sdp.rs                  Offer sanitation + NVST answer blob construction
    peer.rs                 Sans-I/O WebRTC peer: ICE/DTLS/SRTP, RTP → H.264 access units
    rtp.rs                  RTP depacketization helpers
    input_protocol.rs       NVST input-channel binary protocol (gamepad, keys, mouse
                            move/wheel, heartbeat)
    stream_prefs.rs         Persisted stream/control preferences (touch modes, PC overlay,
                            sensitivity, opacity, trigger intensity, audio boost, fps, direct
                            NVIDIA login, etc.)
    regions.rs              Streaming zone/region selection and pinning
    providers.rs            GFN login-provider discovery, and the direct-NVIDIA-login override
                            that skips it (see "Sign-in routing" above)
    error_codes.rs           GeForce NOW error-code catalog (English + Spanish wording)
    link_estimate.rs         Remembered link quality → bitrate ceiling estimation
    queue_stats.rs           Session queue position tracking
    headers.rs               Shared HTTP headers for GFN API calls
docs/protocol-notes.md   GFN protocol reverse-engineering notes (Phase 0)
```

The assets in `static/sce_sys/` (icon, LiveArea backgrounds) are auto-generated solid-color
placeholders — replace them with real art before distributing a VPK.

## Acknowledgements

- [green-vita](https://github.com/Day-OS/green-vita) — the direct-texture video pipeline,
  the Vita-patched `ring`/`rtc-shared` forks, and proof that cloud gaming on a Vita is
  possible at all.
- [OpenNOW](https://github.com/OpenCloudGaming/OpenNOW) and OpenNOW-Switch — the GFN protocol
  reference (CloudMatch, NVST signaling, the input-channel wire format including the mouse
  wheel packet, and the GeForce NOW error-code catalog).
- MattKC's [Vanilla](https://github.com/vanilla-wiiu/vanilla) — the single-reference-frame
  decoder trick.