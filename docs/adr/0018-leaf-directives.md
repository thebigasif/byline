# 0018. Blocks with no body are one-line leaf directives

**Status:** Accepted, 2026-10-26. Phase 1 task 1.7, extending ADR 0012.

## Context

ADR 0012 writes every block beyond markdown as a fenced container: `:::name{...}`, a markdown body, then `:::`. Some blocks have no body at all: a table of contents, a button, a bookmark, an embed. Writing them as containers forces an empty body and a closing line that mean nothing.

## Decision

A block with no body is a leaf directive on one line, `::name{...}`, with two colons, as remark-directive writes leaf directives:

```
::toc
::button{href="/pricing/" label="See prices"}
```

Each block has one form. The leaves today are `button` and `toc`; `bookmark` and `embed` join them in task 1.7. Every other block stays a container. A block written in the wrong form is refused with the right one: "toc has no body; write it on one line as ::toc", or "toggle has a body; open it with :::toggle and close it with :::".

## Consequences

- The same rules as ADR 0012 apply otherwise: attributes in braces, values quoted when they hold a space, unknown blocks and attributes refused with the line.
- A writer coming from remark-directive or MyST already knows both forms.
- The list of leaves lives in one place in `crates/khalid-content/src/directive.rs`.
