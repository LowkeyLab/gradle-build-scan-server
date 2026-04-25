import starlight from "@astrojs/starlight";
import { defineConfig } from "astro/config";

function normalizeBasePath(value) {
  if (!value) {
    return "/";
  }

  const trimmed = value.replace(/^\/+|\/+$/g, "");
  return trimmed ? `/${trimmed}/` : "/";
}

const isDevServer = process.argv.includes("dev");
const productionSite = "https://lowkeylab.github.io/gradle-build-scan-server/";
const productionBase = "/gradle-build-scan-server/";
const configuredSite = process.env.DOCS_SITE || productionSite;
const configuredBase = process.env.DOCS_BASE
  ? normalizeBasePath(process.env.DOCS_BASE)
  : productionBase;

export default defineConfig({
  integrations: [
    starlight({
      title: "Gradle Build Scan Server",
      description: "Project documentation for Gradle Build Scan Server.",
      sidebar: [
        { label: "Overview", link: "/" },
        { slug: "local-build-scan-ui" },
        { slug: "bazel" },
      ],
    }),
  ],
  output: "static",
  site: isDevServer ? undefined : configuredSite,
  base: isDevServer ? "/" : configuredBase,
  trailingSlash: "always",
});
