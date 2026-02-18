# AGENTS.md (Gradle Build Scan Server)

This file contains instructions for AI agents working in this Bazel-based project.

## Development Workflow

1. Make code changes
2. Run `bazel run gazelle`
3. Run `bazel run //tools/format`
4. Run tests: `aspect test //...`
5. Verify build: `aspect build //...`

## Hot Reload

```bash
# Use ibazel for hot-reload during development (auto-rebuilds on file changes)
ibazel run //path/to:target
```

## General Rules

- **Bazel:** MUST run `bazel run gazelle` after editing any source file (.rs, .kt, BUILD, etc.)
- **Formatting:** Run `bazel run //tools/format` before committing
- **Verification:** Run `aspect build //...` to verify changes compile
- **Security:** NEVER hardcode secrets; use environment variables
