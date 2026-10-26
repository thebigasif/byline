# 0006. Where the command line writes, and the shape of its JSON

**Status:** Accepted, 2026-10-26. A technical decision the plan leaves open, made in task 0.2.

## Context

The plan says every command takes `--json` and every error says what, why and fix (BL-CLI-1, BL-CLI-3), and RULES.md fixes the exit codes (BL-CLI-2). It does not say which stream each kind of output uses, or what the JSON looks like when a command fails. Agents such as Claude Code run Byline and read its output, so this has to be one fixed contract.

## Decision

- **Human output.** Results go to standard output. Messages go to standard error as three lines: what, why, fix.
- **JSON output.** With `--json`, everything goes to standard output as one JSON value on one line, so a script reads one stream. A failure is `{"error": {"what": "...", "why": "...", "fix": "..."}}`, the same three fields `UserMessage` serialises to and the API's problem details carry. `--version --json` prints `{"name": "byline", "version": "..."}`.
- **The exit code always tells success from failure,** in both modes: 0 success, 1 a block the person must act on, 2 a command line Byline could not read, 3 a fault inside Byline.
- **Usage errors are messages too.** When the command line cannot be read, Byline says what it could not read, that nothing has changed, and to run `byline --help`. `--json` is honoured even then, as long as it appears among the arguments.

## Consequences

- A script checks the exit code first, then parses standard output when it asked for JSON.
- Commands that succeed will add their own result fields next to these in later phases. A failure always has the `error` object, so a client can test for it.
- Standard error stays free for logs from `tracing` when JSON is on.
