#!/bin/sh
# Builds the Vita binary inside the VitaSDK container, the same toolchain CI uses.
#
# Meant to be run *inside* that container (see scripts/Dockerfile.build): it is what makes a local
# build reproducible without installing VitaSDK, a nightly Rust and cargo-vita on the host.
#
#   docker build -t opennow-build - < scripts/Dockerfile.build
#   docker volume create opennow-cargo
#   docker run --rm -v "$PWD:/work" -v opennow-cargo:/root/.cargo/registry -w /work \
#     opennow-build sh scripts/docker-build.sh vpk
#
# Keep the registry volume: without it every run re-downloads the crates and cargo rebuilds the
# world, which is sixteen minutes instead of three.
set -u
cd "$(dirname "$0")/.."

# A Windows checkout with core.autocrlf on rewrites the VitaSDK wrapper scripts with CRLF endings.
# That turns their shebang into `/bin/sh\r`, and the kernel then reports "No such file or directory"
# for a file that is plainly sitting there - a genuinely confusing hour to lose. `.gitattributes`
# pins them to LF for new checkouts; this repairs an existing one.
sed -i 's/\r$//' tools/vita-ar tools/vita-gcc tools/vita-pkg-config tools/vita-tool Makefile 2>/dev/null
sed -i 's/\r$//' scripts/*.sh 2>/dev/null
chmod +x tools/vita-ar tools/vita-gcc tools/vita-pkg-config tools/vita-tool 2>/dev/null
chmod +x scripts/*.sh 2>/dev/null

# The repo arrives as a bind mount owned by another uid, which git refuses to read by default.
# Without this, build.rs stamps the binary "unknown" and a log can no longer name its commit.
git config --global --add safe.directory /work 2>/dev/null || true

TARGET_STEP="${1:-vpk}"

# The host tests need a host target explicitly: `.cargo/config.toml` pins `[build] target` to the
# Vita, so a bare `cargo test` tries to run the test harness on an ARM binary and fails with
# "can't find crate for `std`" long before it compiles anything.
# A gate, not a report. Piping into `tail` discarded the exit status, so a red suite could not
# stop a build - which is how "73 tests pass" ended up in release notes for a binary whose tests
# nothing had checked.
echo "=== host tests (opennow-core) ==="
if ! cargo test -p opennow-core --target x86_64-unknown-linux-gnu; then
    echo "host tests failed - refusing to build a VPK from this tree" >&2
    exit 1
fi

# Delegated to the Makefile on purpose rather than calling `cargo vita` directly: cargo-vita does
# not forward `.cargo/config.toml`'s rustflags to rustc, so the link flags only take effect when
# they come in through the environment. The Makefile is the one place that spells them out.
echo "=== vita build: $TARGET_STEP ==="
exec make "$TARGET_STEP"
