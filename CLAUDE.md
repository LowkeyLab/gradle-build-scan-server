# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Build & Run

```bash
aspect build //...                       # Build all targets
aspect build //echo-server/src:main      # Build a specific target
aspect run //echo-server/src:main        # Run the echo server
ibazel run //echo-server/src:main        # Hot-reload during development
```

### Test

```bash
aspect test //...           # Run all tests
```

### Format & Lint

```bash
bazel run //tools/format    # Format all files (Rust + Starlark)
aspect lint --fix           # Run clippy with auto-fix
```

### Dependency Management

After editing `Cargo.toml` or adding Rust source files, regenerate BUILD files:

```bash
bazel run gazelle           # MUST run after editing .rs, BUILD, or other source files
```

## Architecture

This is a **Bazel-based Rust monorepo** targeting a Gradle Build Scan server.

### Build system

- **Bazel** with `rules_rust` and `rules_rs` for Cargo crate integration.
- **Gazelle** with `gazelle_rust` plugin auto-generates `BUILD.bazel` files from Rust source — always run `bazel run gazelle` after source changes.
- Crates are sourced from `Cargo.toml`/`Cargo.lock` via `@crates//` label prefix.
- `MODULE.bazel` is the Bzlmod dependency manifest.
- `//tools:bazel_env` exports dev tools (`format`,  `buildifier`) to a `bin/` tree for PATH use via `direnv`.

### Pre-commit hook

Located at `githooks/pre-commit`. Automatically formats staged files on commit. If the formatter modifies staged files, the commit is rejected — stage the formatting changes and commit again.

## Workflow

1. Edit source files
2. `bazel run gazelle` (if `.rs` or `BUILD` files changed)
3. `bazel run //tools/format`
4. `aspect test //...`
5. `aspect build //...`
