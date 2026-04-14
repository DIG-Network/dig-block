//! STR-001: `Cargo.toml` dependencies and minimum versions ([spec](docs/requirements/domains/crate_structure/specs/STR-001.md), [NORMATIVE](docs/requirements/domains/crate_structure/NORMATIVE.md)).
//!
//! ## What this proves
//!
//! - **Dependency closure:** The 13 protocol/build crates listed in STR-001 are present so `dig-block` can use Chia types,
//!   CLVM (`dig-clvm`), and serde/bincode without ad hoc forks ([SPEC §1.2](docs/resources/SPEC.md)).
//! - **Version floor:** Pinned minor versions match the repo’s supported ecosystem (chia0.26, sdk 0.30, etc.) — drift breaks CI early.
//! - **Serde:** The `derive` feature is enabled so block types can use `#[derive(Serialize, Deserialize)]`.
//! - **Linkability:** Importing representative types from `chia-protocol` / `chia-bls` in this binary proves the graph resolves (`cargo check`).

use std::fs;
use std::path::Path;

/// Read workspace `Cargo.toml` for string assertions (integration test has `CARGO_MANIFEST_DIR`).
fn read_cargo_toml() -> String {
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    fs::read_to_string(manifest_path).expect("Failed to read Cargo.toml")
}

#[test]
fn dep_presence_all_13_dependencies_listed() {
    let cargo_toml = read_cargo_toml();

    let required_deps = [
        "chia-protocol",
        "chia-bls",
        "dig-clvm",
        "chia-consensus",
        "chia-sdk-types",
        "chia-sdk-signer",
        "chia-sha2",
        "chia-traits",
        "clvm-utils",
        "clvmr",
        "bincode",
        "serde",
        "thiserror",
    ];

    for dep in &required_deps {
        assert!(
            cargo_toml.contains(dep),
            "Missing required dependency: {dep}"
        );
    }
}

#[test]
fn dep_versions_chia_ecosystem_at_minimum() {
    let cargo_toml = read_cargo_toml();

    // Chia core crates must be >= 0.26
    let chia_026_crates = [
        "chia-protocol",
        "chia-bls",
        "chia-consensus",
        "chia-sha2",
        "chia-traits",
        "clvm-utils",
    ];
    for dep in &chia_026_crates {
        let pattern = format!("{dep} = \"0.26\"");
        assert!(
            cargo_toml.contains(&pattern),
            "Dependency {dep} must be at version 0.26, not found: {pattern}"
        );
    }

    // SDK crates must be >= 0.30
    let sdk_030_crates = ["chia-sdk-types", "chia-sdk-signer"];
    for dep in &sdk_030_crates {
        let pattern = format!("{dep} = \"0.30\"");
        assert!(
            cargo_toml.contains(&pattern),
            "Dependency {dep} must be at version 0.30, not found: {pattern}"
        );
    }

    // clvmr must be >= 0.14
    assert!(
        cargo_toml.contains("clvmr = \"0.14\""),
        "clvmr must be at version 0.14"
    );

    // dig-clvm must be >= 0.1
    assert!(
        cargo_toml.contains("dig-clvm = \"0.1\""),
        "dig-clvm must be at version 0.1"
    );
}

#[test]
fn serde_derive_feature_present() {
    let cargo_toml = read_cargo_toml();
    assert!(
        cargo_toml.contains("features = [\"derive\"]"),
        "serde must include the 'derive' feature"
    );
}

#[test]
fn cargo_check_imports_resolve() {
    // If this test compiles, cargo check succeeded. Verify we can reference
    // key types from each dependency.
    use chia_bls::Signature;
    use chia_protocol::Bytes32;

    let _ = Bytes32::default();
    let _ = Signature::default();
}
