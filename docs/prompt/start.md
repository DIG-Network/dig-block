# Start

## Immediate Actions

1. **Sync**
   ```bash
   git fetch origin && git pull origin main
   ```

2. **Check tools — ALL THREE MUST BE FRESH**
   ```bash
   npx gitnexus status          # GitNexus index fresh?
   npx gitnexus analyze         # Update if stale
   # SocratiCode: verify Docker running, index current
   codebase_status {}            # SocratiCode MCP status
   ```
   **Do not proceed until tools are confirmed operational.** Coding without tools leads to redundant work and missed dependencies.

3. **Pick work** — open `docs/requirements/IMPLEMENTATION_ORDER.md`
   - Choose the first `- [ ]` item
   - Every `- [x]` is done on main — skip it
   - Work phases in order: Phase 0 before Phase 1, etc.

4. **Pack context — BEFORE reading any code**
   ```bash
   npx repomix@latest src -o .repomix/pack-src.xml
   npx repomix@latest tests -o .repomix/pack-tests.xml
   ```

5. **Search with SocratiCode — BEFORE reading files**
   ```
   codebase_search { query: "block header validation builder signer merkle" }
   codebase_graph_query { filePath: "src/block/builder.rs" }
   ```

6. **Read spec** — follow the full trace:
   - `NORMATIVE.md#PREFIX-NNN` → authoritative requirement
   - `specs/PREFIX-NNN.md` → detailed specification + **test plan**
   - `VERIFICATION.md` → how to verify
   - `TRACKING.yaml` → current status

7. **Continue** → [dt-wf-select.md](tree/dt-wf-select.md)

---

## Hard Requirements

1. **Use chia crate ecosystem first** — never reimplement what `chia-protocol`, `chia-bls`, `chia-consensus`, `chia-sdk-types`, `chia-sdk-signer`, `chia-sha2`, `chia-traits`, `clvm-utils` provide. The SPEC Section 1.2 lists every type reused from Chia crates.
2. **Use dig-clvm for CLVM execution** — never call `chia-consensus::run_spendbundle()` directly. Use `dig_clvm::validate_spend_bundle()` which wraps it with DIG-specific configuration.
3. **No custom Condition enum** — use `chia-sdk-types::Condition` directly (43 variants).
4. **No custom CoinRecord** — use `chia-protocol::CoinState` directly via the `CoinLookup` trait.
5. **No custom Merkle set** — use `chia-consensus::compute_merkle_set_root()` for additions/removals roots.
6. **No custom SHA-256** — use `chia-sha2::Sha256` for all hashing.
7. **No custom tree_hash** — use `clvm-utils::tree_hash()` for puzzle hash verification.
8. **Bincode serialization** — all block types use bincode, not Streamable (BLS types don't implement it).
9. **Single-block scope** — this crate never maintains state across blocks, never reads from a database, never makes network calls.
10. **TEST FIRST (TDD)** — write the failing test before writing implementation code. The test defines the contract. The spec's Test Plan section tells you exactly what tests to write.
11. **One requirement per commit** — don't batch unrelated work.
12. **Update tracking after each requirement** — VERIFICATION.md, TRACKING.yaml, IMPLEMENTATION_ORDER.md.
13. **Follow the decision tree to completion** — no shortcuts.
14. **BlockBuilder must produce structurally valid blocks by construction** — `validate_structure()` must always pass on builder output.

---

## Tech Stack

| Component | Crate | Version |
|-----------|-------|---------|
| Protocol types | `chia-protocol` | 0.26 |
| BLS cryptography | `chia-bls` | 0.26 |
| CLVM execution | `dig-clvm` | 0.1 |
| Block validation primitives | `chia-consensus` | 0.26 |
| High-level types | `chia-sdk-types` | 0.30 |
| Signature extraction | `chia-sdk-signer` | 0.30 |
| SHA-256 | `chia-sha2` | 0.26 |
| Streamable trait | `chia-traits` | 0.26 |
| Puzzle hashing | `clvm-utils` | 0.26 |
| CLVM runtime | `clvmr` | 0.14 |
| Serialization | `bincode` | latest |
| Serde framework | `serde` | 1 |
| Error derivation | `thiserror` | latest |
| Testing | `tempfile` | 3 |
