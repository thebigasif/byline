// @ts-check

// The blog lives in the same folder Khalid's `mount` names in site.toml, so
// every address in a post's head, which Khalid writes, is this app's own.
/** @type {import("next").NextConfig} */
const config = {
  basePath: "/blog",
  trailingSlash: true,
};

export default config;
