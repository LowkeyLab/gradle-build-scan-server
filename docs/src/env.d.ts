/// <reference types="astro/client" />

// biome-ignore lint/correctness/noUnusedVariables: ambient Astro env typing
interface ImportMetaEnv {
  readonly DOCS_SITE?: string;
  readonly DOCS_BASE?: string;
}
