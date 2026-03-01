"""
Angular macros.
"""

load("@npm//angular:postcss-cli/package_json.bzl", postcss_cli = "bin")
load("@rules_angular//src/architect:ng_application.bzl", orig_ng_application = "ng_application")
load("@rules_angular//src/architect:ng_test.bzl", orig_ng_test = "ng_test")

def process_styles(name, src, out, srcs = [], config = "//angular:postcssrc", deps = [], visibility = None):
    """Processes a CSS file using PostCSS with Tailwind CSS support."""
    postcss_cli.postcss(
        name = name,
        visibility = visibility,
        srcs = [src, config] + deps + srcs + [
            "//angular:node_modules/@tailwindcss/postcss",
            "//angular:node_modules/postcss",
            "//angular:node_modules/tailwindcss",
        ],
        outs = [out],
        args = [
            "$(rootpath {})".format(src),
            "-o",
            "$(rootpath {})".format(out),
            "--config",
            "$(rootpath {})".format(config),
        ],
    )

def ng_application(name, srcs = [], deps = [], zonejs = False, tailwindcss = False, visibility = None):
    """Defines an ng_application with optional dependencies on zone.js and tailwindcss."""
    extra_deps = []
    if zonejs:
        extra_deps.append("//angular:node_modules/zone.js")
    if tailwindcss:
        extra_deps += [
            "//angular:node_modules/@tailwindcss/postcss",
            "//angular:node_modules/postcss",
            "//angular:node_modules/tailwindcss",
            "//angular:postcssrc",
        ]
    orig_ng_application(
        name = name,
        visibility = visibility,
        srcs = srcs,
        deps = deps + extra_deps,
        ng_config = "//angular:ng-config",
        node_modules = "//angular:node_modules",
    )

def ng_test(name, srcs = [], deps = [], zonejs = False, tailwindcss = False, karma = False, vitest = False, visibility = None):
    """Defines an ng_test with optional dependencies on zone.js, tailwindcss, karma, and vitest."""
    extra_deps = []
    if zonejs:
        extra_deps.append("//angular:node_modules/zone.js")
    if tailwindcss:
        extra_deps += [
            "//angular:node_modules/@tailwindcss/postcss",
            "//angular:node_modules/postcss",
            "//angular:node_modules/tailwindcss",
            "//angular:postcssrc",
        ]
    if karma:
        extra_deps += [
            "//angular:node_modules/@types/jasmine",
            "//angular:node_modules/@types/node",
            "//angular:node_modules/jasmine-core",
            "//angular:node_modules/karma",
            "//angular:node_modules/karma-chrome-launcher",
            "//angular:node_modules/karma-coverage",
            "//angular:node_modules/karma-jasmine",
            "//angular:node_modules/karma-jasmine-html-reporter",
        ]
    if vitest:
        extra_deps += [
            "//angular:node_modules/jsdom",
            "//angular:node_modules/vitest",
        ]

    orig_ng_test(
        name = name,
        visibility = visibility,
        srcs = srcs,
        deps = deps + extra_deps,
        ng_config = "//angular:ng-config",
        node_modules = "//angular:node_modules",
        size = "small",
    )
