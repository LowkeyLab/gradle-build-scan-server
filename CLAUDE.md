# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Build & Run

```bash
aspect build //...                       # Build all targets
aspect build //build-scan/server/src:main # Build a specific target
bazel run //build-scan/server/src:main   # Run the build scan server
ibazel run //build-scan/server/src:main  # Hot-reload during development
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

### Subsystems

| Directory | Purpose |
|-----------|---------|
| `build-scan/` | Binary format parser (lib), CLI, and HTTP server (ingest, GraphQL, SPA) |
| `proxy/` | HTTP intercepting proxy capturing Gradle client traffic to SQLite |
| `angular/` | Angular 21 SPA frontend (Tailwind + DaisyUI + Apollo) |
| `gradle/` | Dogfooding Gradle project that publishes scans to the server |

### Build system

- **Bazel** with `rules_rust` and `rules_rs` for Cargo crate integration.
- **Gazelle** with `gazelle_rust` plugin auto-generates `BUILD.bazel` files from Rust source — always run `bazel run gazelle` after source changes. **Important**: Gazelle may strip internal workspace deps it cannot resolve (ambiguous crate names across `build-scan/` and `proxy/`). Use `# keep` comments on these deps and always verify with a build after running gazelle.
- Crates are sourced from `Cargo.toml`/`Cargo.lock` via `@crates//` label prefix.
- `MODULE.bazel` is the Bzlmod dependency manifest.
- `//tools:bazel_env` exports dev tools (`format`,  `buildifier`) to a `bin/` tree for PATH use via `direnv`.

### Worktree Builds

Bazel remote cache can serve stale artifacts when building in a git worktree. Use `--noremote_accept_cached --disk_cache=""` after `bazel clean` to force local compilation.

### Pre-commit hook

Located at `githooks/pre-commit`. Automatically formats staged files on commit. If the formatter modifies staged files, the commit is rejected — stage the formatting changes and commit again.

## Workflow

1. Edit source files
2. `bazel run gazelle` (if `.rs` or `BUILD` files changed)
3. `bazel run //tools/format`
4. `aspect test //...`
5. `aspect build //...`

### Dogfooding

Generate test build scans by running the Gradle project against your local server:

```bash
cd gradle && DEVELOCITY_SERVER_URL=http://localhost:3000 ./gradlew clean build
```
