# 0008. YAML front matter, read with serde-saphyr

**Status:** Accepted, 2026-10-26. Phase 1 task 1.5, with the CTO's decision on the format.

## Context

Every post starts with front matter: the title, date, author and the rest. The format has to be the one writers already know, and the parser has to be maintained, permissive and safe, because it reads every content file and every import.

## Decision

Front matter is YAML between two `---` lines, the form WordPress exports, Hugo, Jekyll and Astro writers use, and the demo site already follows. TOML front matter (`+++`) is accepted only by the markdown-folder importer in Phase 6, which rewrites it to YAML.

It is read with `serde-saphyr` 2: pure Rust, MIT or Apache-2.0, actively maintained, with line and column on every error. The alternatives and why each was refused:

| Crate | Why not |
|---|---|
| `serde_yaml` | Archived by its author in 2024 and no longer maintained |
| `serde_yml` | Deprecated by its author; now a compatibility shim that forwards to another library |
| `serde_yaml_ng` | Maintained, but built on `unsafe-libyaml`, a C library translated to unsafe Rust; the workspace forbids `unsafe`, and the parser of untrusted files is the last place to accept it in a dependency |
| `serde_norway` | A fork of `serde_yaml` on the same `unsafe-libyaml` base |

## Consequences

- Front matter errors name the file, the line and the field. The parser's own wording is kept where it is plain, and replaced where it is not: a bad date says it must be a real day written as YYYY-MM-DD.
- If `serde-saphyr` stops being maintained, this record is superseded with the replacement and a reason.
