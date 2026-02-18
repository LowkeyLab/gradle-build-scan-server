"""Targets in the repository root"""

load("@gazelle//:def.bzl", "DEFAULT_LANGUAGES", "gazelle", "gazelle_binary")

exports_files(
    [
        ".clippy.toml",
    ],
    visibility = ["//:__subpackages__"],
)

gazelle_binary(
    name = "gazelle_bin",
    languages = DEFAULT_LANGUAGES + [
        "@bazel_skylib_gazelle_plugin//bzl",
        "@gazelle_rust//rust_language",
    ],
)

gazelle(
    name = "gazelle",
    gazelle = ":gazelle_bin",
)
