// Integration test for SigmaOS-arch-audit
// Verifies the crate builds and links as a dependency.
#![allow(unused_imports)]

use sigmaos_arch_audit as _;

#[test]
fn crate_links_and_runs() {
    assert_eq!(2 + 2, 4);
}
