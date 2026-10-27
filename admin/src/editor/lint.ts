import type { Problem } from "../api/posts";

/**
 * One lint finding, as `khalid lint --json` and the Admin API print it. The
 * OpenAPI document gives a refused publish's `lint` as a plain object, so
 * this narrows it field by field rather than trusting it.
 */
export type Finding = {
  rule: string;
  severity: "block" | "warn";
  subject: string | null;
  what: string;
  why: string;
  fix: string;
};

function isFinding(value: unknown): value is Finding {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const texts = ["rule", "what", "why", "fix"].every(
    (name) => name in value && typeof Reflect.get(value, name) === "string",
  );
  const severity = "severity" in value ? value.severity : undefined;
  const subject = "subject" in value ? value.subject : null;
  return (
    texts &&
    (severity === "block" || severity === "warn") &&
    (subject === null || typeof subject === "string")
  );
}

/** The findings behind a refused publish, blocks first; none for any other problem. */
export function lintFindings(problem: Problem | null): Finding[] {
  const lint = problem?.lint;
  if (lint === null || lint === undefined || !Array.isArray(lint.findings)) {
    return [];
  }
  const findings: unknown[] = lint.findings;
  const blocks = findings.filter(isFinding).filter((finding) => finding.severity === "block");
  const warnings = findings.filter(isFinding).filter((finding) => finding.severity === "warn");
  return [...blocks, ...warnings];
}
