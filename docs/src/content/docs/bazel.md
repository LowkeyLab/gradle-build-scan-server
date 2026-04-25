---
title: Bazel workflows
description: Contributor workflows for formatting, linting, and working with Bazel in this repository.
---

This repository uses [Aspect Workflows](https://aspect.build) to provide an excellent Bazel developer experience.

The main project overview and runtime setup live on the [documentation home page](../).

## Formatting code

The `format` command is provided by Bazel via the `//tools/format` target.

- Run `bazel run //tools/format` to re-format all files locally.
- Run `bazel run //tools/format -- path/to/file` to re-format a single file.
- Run `git config core.hooksPath githooks` to add the formatter pre-commit hook.
- For CI verification, set up the `format` task in CI; see https://docs.aspect.build/workflows/features/lint#formatting

## Linting code

Projects use [rules_lint](https://github.com/aspect-build/rules_lint) to run linting tools using Bazel's aspects feature.
Linters produce report files, which they cache like any other Bazel actions.
You can print the report files to the terminal in a couple ways, as follows.

The Aspect CLI provides the [`lint` command](https://docs.aspect.build/cli/commands/aspect_lint) but it is _not_ part of the Bazel CLI provided by Google.
The command collects the correct report files, presents them with colored boundaries, gives you interactive suggestions to apply fixes, produces a matching exit code, and more.

- Run `aspect lint //...` to check for lint violations.

## Installing dev tools

Dev tools are provided by the Nix dev shell (see `flake.nix`). Tools are automatically available on PATH via direnv.

1. Install Nix with flakes enabled
2. Run `direnv allow` in the workspace
3. Tools will be available on the PATH automatically

## Working with Cargo

You can run `cargo` outside of Bazel, using the tool installed on the PATH.

```console
% cargo add reqwest
    Updating crates.io index
      Adding reqwest v0.12.7 to dependencies.
             Features:
             + __tls
             + charset
             + default-tls
             + h2
             + http2
             + macos-system-configuration
             25 deactivated features
    Updating crates.io index
```
