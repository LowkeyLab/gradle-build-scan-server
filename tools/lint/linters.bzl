"Define linter aspects"

load("@aspect_rules_lint//lint:clippy.bzl", "lint_clippy_aspect")
load("@aspect_rules_lint//lint:eslint.bzl", "lint_eslint_aspect")
load("@aspect_rules_lint//lint:lint_test.bzl", "lint_test")

clippy = lint_clippy_aspect(
    config = Label("//:.clippy.toml"),
)

eslint = lint_eslint_aspect(
    binary = Label(":eslint"),
    configs = [
        Label("//:eslintrc"),
    ],
)

eslint_test = lint_test(aspect = eslint)
