.PHONY: vpk desktop eboot sync-version

# `cargo-vita` does not forward `.cargo/config.toml`'s rustflags through to rustc, so the flags
# that matter have to be here too - see the NOTE in that file.
#
# The three `link-arg`s: SDL2 pulls in libvitaGL and libvitashark, which are C++ and reference the
# Cg shader extension stubs, and neither was being linked. As of the September 2026
# `vitasdk/vitasdk` image that makes the link step fail with a wall of `undefined reference to
# std::__throw_length_error` and `sceShaccCgExtEnableExtensions`. They go last so ld resolves them
# after the libraries that ask for them.
RUSTFLAGS ?= -C target-feature=-neon -C link-arg=-lSceShaccCgExt -C link-arg=-lSceShaccCg_stub -C link-arg=-lstdc++ -C link-arg=-ltaihen_stub_weak
CARGO_VITA ?= cargo +nightly vita

VPK := target/armv7-sony-vita-newlibeabihf/release/opennow-vita.vpk
DESKTOP_DIR ?= $(HOME)/Desktop

# Keeps the VPK's APP_VER lined up with [package].version - see scripts/sync-vita-version.sh.
sync-version:
	@./scripts/sync-vita-version.sh

vpk: sync-version
	RUSTFLAGS="$(RUSTFLAGS)" $(CARGO_VITA) build vpk --release

# Release build copied to the Desktop for manual installation.
desktop: vpk
	cp $(VPK) $(DESKTOP_DIR)/opennow-vita.vpk
	@ls -lh $(DESKTOP_DIR)/opennow-vita.vpk

eboot: sync-version
	RUSTFLAGS="$(RUSTFLAGS)" $(CARGO_VITA) build eboot --release
