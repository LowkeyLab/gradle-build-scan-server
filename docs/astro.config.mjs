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
const configuredSite = process.env.DOCS_SITE;
const configuredBase = normalizeBasePath(process.env.DOCS_BASE);

export default defineConfig({
  integrations: [
    starlight({
      title: "Gradle Build Scan Server",
      description:
        "Project documentation for Gradle Build Scan Server.",
      sidebar: [
        { label: "Overview", link: "/" },
        { slug: "bazel" },
      ],
    }),
  ],
  output: "static",
  site: configuredSite || undefined,
  base: isDevServer ? "/" : configuredBase,
  trailingSlash: "always",
});
