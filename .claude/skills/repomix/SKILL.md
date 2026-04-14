# Repomix — Context Packing Skill

## When to Use

Use Repomix **before implementing any requirement**. Pack the relevant scope so the LLM has full awareness of the code being modified.

## HARD RULE

**MUST pack context before writing implementation code.** Fresh context prevents redundant work and missed patterns.

## Commands

### Pack Implementation

```bash
npx repomix@latest src -o .repomix/pack-src.xml
```

### Pack Tests (CRITICAL for TDD)

```bash
npx repomix@latest tests -o .repomix/pack-tests.xml
```

### Pack Requirements by Domain

```bash
# attestation
npx repomix@latest docs/requirements/domains/attestation -o .repomix/pack-attestation-reqs.xml

# block_production
npx repomix@latest docs/requirements/domains/block_production -o .repomix/pack-block-production-reqs.xml

# block_types
npx repomix@latest docs/requirements/domains/block_types -o .repomix/pack-block-types-reqs.xml

# checkpoint
npx repomix@latest docs/requirements/domains/checkpoint -o .repomix/pack-checkpoint-reqs.xml

# crate_structure
npx repomix@latest docs/requirements/domains/crate_structure -o .repomix/pack-crate-structure-reqs.xml

# error_types
npx repomix@latest docs/requirements/domains/error_types -o .repomix/pack-error-types-reqs.xml

# execution_validation
npx repomix@latest docs/requirements/domains/execution_validation -o .repomix/pack-execution-validation-reqs.xml

# hashing
npx repomix@latest docs/requirements/domains/hashing -o .repomix/pack-hashing-reqs.xml

# receipt
npx repomix@latest docs/requirements/domains/receipt -o .repomix/pack-receipt-reqs.xml

# serialization
npx repomix@latest docs/requirements/domains/serialization -o .repomix/pack-serialization-reqs.xml

# state_validation
npx repomix@latest docs/requirements/domains/state_validation -o .repomix/pack-state-validation-reqs.xml

# structural_validation
npx repomix@latest docs/requirements/domains/structural_validation -o .repomix/pack-structural-validation-reqs.xml

# All requirements at once
npx repomix@latest docs/requirements -o .repomix/pack-requirements.xml
```

### Pack the Full Spec

```bash
npx repomix@latest docs/resources -o .repomix/pack-spec.xml
```

### Pack with Compression

```bash
npx repomix@latest src --compress -o .repomix/pack-src-compressed.xml
```

### Pack Multiple Scopes

```bash
npx repomix@latest src tests -o .repomix/pack-impl-and-tests.xml
```

## Workflow Integration

| Step | Pack Command |
|------|-------------|
| Before writing tests | `npx repomix@latest tests -o .repomix/pack-tests.xml` |
| Before implementing | `npx repomix@latest src -o .repomix/pack-src.xml` |
| Cross-domain work | Pack both domains' requirements |

## Notes

- `.repomix/` is gitignored — pack files are never committed
- Regenerate packs when switching requirements
- Use `--compress` for large scopes to manage token count
- Pack requirements alongside code for spec compliance checks

## Full Documentation

See `docs/prompt/tools/repomix.md` for complete reference.
