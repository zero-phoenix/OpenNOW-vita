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
adds a **PC-Touch Overlay Mod** that turns the Vita's front and rear touchscreens into a
keyboard+mouse for the remote desktop — handy for driving Windows, Steam, or Epic through a
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
- [PC-Touch Overlay Mod](#pc-touch-overlay-mod)
- [Sign-in routing: avoiding regional partner logins](#sign-in-routing-avoiding-regional-partner-logins)
- [Status](#status)
- [Getting a build](#getting-a-build)
- [Build requirements](#build-requirements)
- [Building locally](#building-locally)
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
  screen (superseded by mouse clicks while the PC-Touch Overlay is on — see below).
- **In-game keyboard** — the Vita's inline IME, wired so character/Backspace/Enter/arrow edits
  are forwarded to the stream as real keystrokes.
- **PC-Touch Overlay Mod** — use the Vita as a keyboard+mouse for the remote desktop; see
  [below](#pc-touch-overlay-mod).
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

1. **[PC-Touch Overlay Mod](#pc-touch-overlay-mod)** — full keyboard+mouse control of the
   remote desktop from the Vita's own touchscreens, with physical-button macros, all gated
   behind a single Settings toggle so it never interferes with normal controller play.
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

See `CHANGELOG.md` for the complete, dated history of every change, including everything
inherited from upstream.

## PC-Touch Overlay Mod

This fork adds a mode that turns the Vita's two touchscreens into a keyboard+mouse for
controlling whatever is running on the far end of the stream — for example, the Windows 11
desktop under a GeForce NOW Ultimate **Install-to-Play** session, so you can drive Steam/Epic
game and OS UI without a physical mouse and keyboard nearby. Enable it from **Settings →
Controls → PC overlay**.

- **Rear touch panel (back of the Vita) drives the mouse cursor.** The NVST input protocol
  only has a *relative* mouse-move packet (`INPUT_MOUSE_MOVE_REL`, clamped at ±4096 per axis)
  — there is no absolute-position packet to move a cursor to an exact point on the host
  screen. So the rear panel is deliberately mapped as a **relative trackpad**: dragging your
  finger moves the cursor by a delta, the same way a laptop trackpad works, not a 1:1
  touch-to-pixel map. A configurable DPI/sensitivity multiplier scales that delta.
- **Front screen exposes seven overlay zones** (only active while the PC overlay is on), drawn
  with semi-transparent egui rectangles and readable labels, at a configurable opacity:
  - Top-left corner — **Esc**
  - Top-right corner — opens the app's own **Settings** menu (interrupts game input while open)
  - Left edge, middle — vertical **DPI/sensitivity slider** for the rear trackpad
  - Bottom-left corner — **left click**
  - Bottom-right corner — **right click**
  - Right edge, middle — vertical **scroll slider** (mouse wheel)
  - Right edge, between the scroll slider and the right-click corner — **Enter**
- **Mutually exclusive with L3/R3.** The front screen's bottom corners are also where the
  existing `FrontStickZones` maps L3/R3 for normal controller play. Turning the PC overlay on
  hands those corners to mouse clicks instead; turning it off gives them straight back to
  L3/R3, with no double-input or fighting between the two modes.
- **Scroll wheel is a real NVST packet, not a fallback.** `INPUT_MOUSE_WHEEL` was added to
  `src/gfn/input_protocol.rs`, ported from the OpenNOW/OpenNOW-Switch reference clients' wire
  format — not invented from scratch.
- **Physical-button macros**, active only while the overlay is on:
  - **L trigger (held)** — halves the mouse sensitivity ("sniper mode"), for fine cursor work
  - **SELECT** — toggles the on-screen keyboard
  - **D-Pad Up** — sends **Win+D** (show desktop)
  - **D-Pad Down** — sends **Ctrl+Alt+Del**
- Overlay on/off, opacity, and mouse sensitivity are persistent preferences (`stream_prefs.rs`)
  editable from Settings, same as every other stream/control option.

Relevant source: `src/input.rs` (touch-zone routing, physical-button macros),
`src/gfn/input_protocol.rs` (mouse-move/wheel packet encoding), `src/gfn/stream_prefs.rs`
(persisted overlay prefs), `src/app/settings_menu.rs` and `src/app/ui.rs` (settings rows and
egui overlay rendering).

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

If you don't have VitaSDK installed natively, the whole build works unmodified inside the
official container image:

```sh
docker run --rm -v "$PWD:/workspace" -w /workspace -e VITASDK=/usr/local/vitasdk \
  vitasdk/vitasdk:latest bash -c '
    curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly --profile minimal
    export PATH="/usr/local/vitasdk/bin:$HOME/.cargo/bin:$PATH"
    rustup component add rust-src --toolchain nightly
    cargo +nightly install cargo-vita
    make vpk
  '
```

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
  input.rs               SDL2 event mapping (keyboard/controller/touch), front/rear touch
                          zone routing (PC overlay zones, FrontStickZones, trackpad), physical
                          button macros, XInput snapshots
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
