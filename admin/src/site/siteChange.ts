import type { SiteChange, SiteSettings } from "../api/site";

/** The settings as the form holds them: languages as codes split by commas. */
export type SiteFields = {
  name: string;
  address: string;
  mount: string;
  default_language: string;
  languages: string;
};

/** The form's starting text for `site`. */
export function fieldsOf(site: SiteSettings): SiteFields {
  return {
    name: site.name,
    address: site.address,
    mount: site.mount,
    default_language: site.default_language,
    languages: site.languages.join(", "),
  };
}

/** The language codes in a comma-separated list, in order, each once. */
export function languageCodes(text: string): string[] {
  const codes = text.split(",").map((code) => code.trim().toLowerCase());
  return [...new Set(codes.filter((code) => code !== ""))];
}

/** The change that turns `site` into `fields`, only what changed; `null` when nothing did. */
export function siteChange(site: SiteSettings, fields: SiteFields): SiteChange | null {
  const before = fieldsOf(site);
  const change: SiteChange = { updated_at: site.updated_at };
  let changed = false;
  for (const key of ["name", "address", "mount", "default_language"] as const) {
    if (fields[key].trim() !== before[key]) {
      change[key] = fields[key].trim();
      changed = true;
    }
  }
  const languages = languageCodes(fields.languages);
  if (languages.join(",") !== site.languages.join(",")) {
    change.languages = languages;
    changed = true;
  }
  return changed ? change : null;
}
