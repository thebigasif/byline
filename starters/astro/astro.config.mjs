// @ts-check
import { defineConfig } from "astro/config";

// The blog lives in the same folder Khalid's `mount` names in site.toml, so
// every address in a post's head, which Khalid writes, is this site's own.
export default defineConfig({
  site: process.env.SITE_ADDRESS ?? "https://www.example.com",
  base: "/blog",
  trailingSlash: "always",
  output: "static",
});
