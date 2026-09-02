//! Gate: counter product crates + shell helpers are workspace members.
//!
//! Featureless sibling-source contract (gauge / lepton-shell pattern).

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn counter_product_workspace_members_happy_path() {
    let root =
        fs::read_to_string(workspace_root().join("Cargo.toml")).expect("workspace Cargo.toml");
    for member in [
        "counter-app",
        "counter-app-worker",
        "counter-app-spectra-topics",
        "examples/local-counter-host",
    ] {
        assert!(
            root.contains(&format!("\"{member}\"")),
            "workspace must list {member}"
        );
        assert!(
            workspace_root().join(member).join("Cargo.toml").is_file(),
            "missing crate dir {member}"
        );
    }
}
