// Reads the Lighthouse reports scripts/demo-check.sh wrote and fails on any
// category under 100 (docs/standards/performance.md). For each low score it
// names the audits that cost points, so the fix can go to the crate that
// owns them.
import { readFileSync } from "node:fs";

let low = 0;
for (const file of process.argv.slice(2)) {
  const report = JSON.parse(readFileSync(file, "utf8"));
  const url = report.finalDisplayedUrl ?? report.requestedUrl;
  if (report.runtimeError) {
    low += 1;
    console.log(`Lighthouse could not check ${url}: ${report.runtimeError.message}`);
    continue;
  }
  for (const category of Object.values(report.categories)) {
    const score = Math.round((category.score ?? 0) * 100);
    console.log(`${score} ${category.title}: ${url}`);
    if (score >= 100) continue;
    low += 1;
    for (const ref of category.auditRefs) {
      const audit = report.audits[ref.id];
      if (ref.weight > 0 && audit.score !== null && audit.score < 1) {
        console.log(`    costs points: ${audit.title} (${ref.id})`);
      }
    }
  }
}

if (low > 0) {
  console.log(`${low} Lighthouse scores are under 100. Each one breaks the budget in docs/standards/performance.md. Fix the audits named above, then run scripts/demo-check.sh again.`);
  process.exit(1);
}
console.log("Every Lighthouse score is 100.");
