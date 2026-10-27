// Fails the build when the admin's scripts and styles, compressed, pass
// the budget in docs/standards/performance.md.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { gzipSync } from "node:zlib";

const BUDGET_BYTES = 350 * 1024;
const DIST = "dist";

function files(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? files(path) : [path];
  });
}

const counted = files(DIST).filter((path) => /\.(js|css|html)$/.test(path));
let total = 0;
for (const path of counted) {
  const size = gzipSync(readFileSync(path), { level: 9 }).length;
  total += size;
  console.log(`${(size / 1024).toFixed(1).padStart(8)} KB  ${path}`);
}

const used = (total / 1024).toFixed(1);
if (total > BUDGET_BYTES) {
  console.error(`The admin is ${used} KB compressed, over its 350 KB budget.`);
  console.error("Split a route with a lazy import, or drop a dependency, then build again.");
  process.exit(1);
}
console.log(`The admin is ${used} KB compressed, inside its 350 KB budget.`);
