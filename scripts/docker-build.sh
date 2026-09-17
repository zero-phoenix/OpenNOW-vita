#!/bin/sh
# Builds the Vita binary inside the VitaSDK container, the same toolchain CI uses.
#
# Meant to be run *inside* that container (see scripts/Dockerfile.build): it is what makes a local
# build reproducible without installing VitaSDK, a nightly Rust and cargo-vita on the host.
#
#   docker build -t opennow-build - < scripts/Dockerfile.build
#   docker run --rm -v "$PWD:/work" -w /work opennow-build sh scripts/docker-build.sh
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

TARGET_STEP="${1:-elf}"
echo "=== host tests (opennow-core) ==="
cargo test -p opennow-core --target x86_64-unknown-linux-gnu 2>&1 | tail -5
echo "=== vita build: $TARGET_STEP ==="
exec cargo +nightly vita build "$TARGET_STEP" --release
