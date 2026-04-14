//! STR-002: Module Hierarchy verification tests.
//!
//! Verifies all required source files exist and module declarations are correct.

use std::path::Path;

fn src_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

#[test]
fn file_existence_all_required_source_files_present() {
    let src = src_dir();

    let required_files = [
        // Top-level modules
        "lib.rs",
        "constants.rs",
        "primitives.rs",
        "error.rs",
        "hash.rs",
        "merkle_util.rs",
        "traits.rs",
        // types/ module
        "types/mod.rs",
        "types/header.rs",
        "types/block.rs",
        "types/attested.rs",
        "types/checkpoint.rs",
        "types/receipt.rs",
        "types/signer_bitmap.rs",
        "types/status.rs",
        // validation/ module
        "validation/mod.rs",
        "validation/structural.rs",
        "validation/execution.rs",
        "validation/state.rs",
        // builder/ module
        "builder/mod.rs",
        "builder/block_builder.rs",
        "builder/checkpoint_builder.rs",
    ];

    for file in &required_files {
        let path = src.join(file);
        assert!(path.exists(), "Missing required source file: {file}");
    }
}

#[test]
fn mod_declarations_types_mod_rs() {
    let content = std::fs::read_to_string(src_dir().join("types/mod.rs"))
        .expect("Failed to read types/mod.rs");

    let expected_modules = [
        "header",
        "block",
        "attested",
        "checkpoint",
        "receipt",
        "signer_bitmap",
        "status",
    ];
    for module in &expected_modules {
        assert!(
            content.contains(&format!("mod {module}"))
                || content.contains(&format!("mod {module};")),
            "types/mod.rs missing module declaration for: {module}"
        );
    }
}

#[test]
fn mod_declarations_validation_mod_rs() {
    let content = std::fs::read_to_string(src_dir().join("validation/mod.rs"))
        .expect("Failed to read validation/mod.rs");

    let expected_modules = ["structural", "execution", "state"];
    for module in &expected_modules {
        assert!(
            content.contains(&format!("mod {module}"))
                || content.contains(&format!("mod {module};")),
            "validation/mod.rs missing module declaration for: {module}"
        );
    }
}

#[test]
fn mod_declarations_builder_mod_rs() {
    let content = std::fs::read_to_string(src_dir().join("builder/mod.rs"))
        .expect("Failed to read builder/mod.rs");

    let expected_modules = ["block_builder", "checkpoint_builder"];
    for module in &expected_modules {
        assert!(
            content.contains(&format!("mod {module}"))
                || content.contains(&format!("mod {module};")),
            "builder/mod.rs missing module declaration for: {module}"
        );
    }
}

#[test]
fn lib_modules_all_top_level_declared() {
    let content = std::fs::read_to_string(src_dir().join("lib.rs")).expect("Failed to read lib.rs");

    let expected_modules = [
        "constants",
        "primitives",
        "error",
        "hash",
        "traits",
        "types",
        "validation",
        "builder",
    ];
    for module in &expected_modules {
        assert!(
            content.contains(&format!("mod {module}"))
                || content.contains(&format!("mod {module};")),
            "lib.rs missing top-level module declaration for: {module}"
        );
    }
}

/// **Project convention:** All requirement integration tests live as **individual `*.rs` files directly under `tests/`**
/// (no `tests/<domain>/` subfolders). Shared helpers are `tests/common.rs`. This keeps `Cargo.toml` `[[test]]` paths
/// uniform and matches tracker/spec references.
#[test]
fn integration_tests_directory_is_flat() {
    let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    for entry in std::fs::read_dir(&tests).expect("read tests/") {
        let entry = entry.expect("directory entry");
        let path = entry.path();
        assert!(
            path.is_file(),
            "tests/ must be flat — no subdirectories (remove {:?})",
            path
        );
        assert_eq!(
            path.extension().and_then(|e| e.to_str()),
            Some("rs"),
            "tests/ should only contain Rust sources: {:?}",
            path
        );
    }
}

#[test]
fn compile_check_module_graph_resolves() {
    // If this test compiles, the module graph is valid.
    // Verify we can access submodules through the crate.
    use dig_block::constants;
    use dig_block::error;

    let _ = constants::EMPTY_ROOT;
    // ERR-001: use a concrete Tier-1 variant so the module graph stays tied to real APIs.
    let _ = error::BlockError::InvalidData(String::new());
}
