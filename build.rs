//! Stamps the commit and build time into the binary.
//!
//! 0.6.0 shipped a VPK of 9,993,553 bytes while the only build anyone had ever run was 9,993,931
//! bytes. Two different binaries, one version number, and no way to tell from a log which of them
//! produced it - so every observation was attached to an unknown artifact. A build id costs four
//! lines and removes that whole class of ambiguity.

use std::process::Command;

fn main() {
    // Only re-run when the commit can actually have changed, not on every source edit.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
    println!("cargo:rerun-if-env-changed=OPENNOW_BUILD_REV");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");

    // CI may hand it to us directly; a shallow or missing checkout must not fail the build.
    // Even in that case, preserve the fact that the tree was dirty. The Makefile supplies the
    // commit for Docker builds, and treating an edited VPK as that clean commit would attach a
    // report to the wrong artifact.
    let rev = std::env::var("OPENNOW_BUILD_REV").ok().or_else(|| {
        let output = Command::new("git")
            .args(["rev-parse", "--short=12", "HEAD"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let rev = String::from_utf8(output.stdout).ok()?.trim().to_owned();
        Some(rev)
    });
    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .is_some_and(|out| !out.stdout.is_empty());
    let rev = rev.map(|rev| {
        if dirty && !rev.ends_with("-dirty") {
            format!("{rev}-dirty")
        } else {
            rev
        }
    });
    println!(
        "cargo:rustc-env=OPENNOW_BUILD_REV={}",
        rev.as_deref().unwrap_or("unknown")
    );

    // SOURCE_DATE_EPOCH keeps this reproducible where the caller wants it to be; otherwise the
    // wall clock is good enough to tell two builds of the same commit apart.
    let built = std::env::var("SOURCE_DATE_EPOCH").ok().unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|_| "unknown".to_owned())
    });
    println!("cargo:rustc-env=OPENNOW_BUILD_TIME={built}");
}
