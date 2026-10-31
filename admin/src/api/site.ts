// The settings routes of the Admin API. They are typed by hand, field for
// field as `AdminSettings` and `SettingsChange` in the G24 stack, because
// this document gains them only when that stack lands
// (docs/decisions/2026-10-31-admin-screens.md). The answer wraps one
// object, `{ "settings": AdminSettings }`, as G24's handlers write it.
import { call, type Reply, sending } from "./call";

/** The site's core settings, as `site.toml` holds them. */
export type SiteSettings = {
  name: string;
  /** The main site's address, such as `https://www.example.com`. */
  address: string;
  /** The path the blog lives under, such as `/blog/`. */
  mount: string;
  default_language: string;
  languages: string[];
  /** When `site.toml` last changed; a change sends it back. */
  updated_at: string;
  /** Whether the running server needs a restart to use what `site.toml` says. */
  restart_needed: boolean;
};

/** A change: only the fields sent change. */
export type SiteChange = {
  updated_at: string;
  name?: string | null;
  address?: string | null;
  mount?: string | null;
  default_language?: string | null;
  languages?: string[] | null;
};

async function settings(response: Response): Promise<SiteSettings> {
  const body: { settings: SiteSettings } = await response.json();
  return body.settings;
}

/** The site's core settings. */
export function readSite(): Promise<Reply<SiteSettings>> {
  return call("/settings/", {}, settings);
}

/** Saves a change made on the copy that `change.updated_at` names. */
export function changeSite(change: SiteChange): Promise<Reply<SiteSettings>> {
  return call("/settings/", sending("PUT", { settings: change }), settings);
}
