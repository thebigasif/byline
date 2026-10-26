// The browser half of scripts/demo-check.sh. Opens every page at six widths,
// first with scripts on and then with scripts off, and fails on any axe
// violation (BL-WEB-4), a page wider than its window (BL-WEB-3), or a page
// that is not whole: no main landmark or not exactly one h1 (BL-WEB-1,
// BL-WEB-2).
import { readFileSync } from "node:fs";
import AxeBuilder from "@axe-core/playwright";
import { chromium } from "playwright";

const WIDTHS = [320, 375, 768, 1024, 1280, 1920];
const HEIGHT = 900;

const pages = readFileSync(process.argv[2], "utf8").split("\n").filter(Boolean);
const failures = [];
const browser = await chromium.launch();
try {
  for (const scripts of [true, false]) {
    const context = await browser.newContext({ javaScriptEnabled: scripts });
    const page = await context.newPage();
    for (const url of pages) {
      for (const width of WIDTHS) {
        const where = `${url} at ${width} px with scripts ${scripts ? "on" : "off"}`;
        await page.setViewportSize({ width, height: HEIGHT });
        await page.goto(url, { waitUntil: "load" });
        const shape = await page.evaluate(() => ({
          wider: document.documentElement.scrollWidth - document.documentElement.clientWidth,
          h1: document.querySelectorAll("h1").length,
          main: document.querySelectorAll("main").length,
        }));
        if (shape.wider > 0) failures.push(`${where}: the page is ${shape.wider} px wider than the window`);
        if (shape.h1 !== 1) failures.push(`${where}: the page has ${shape.h1} h1 headings, not one`);
        if (shape.main !== 1) failures.push(`${where}: the page has ${shape.main} main landmarks, not one`);
        const { violations } = await new AxeBuilder({ page }).analyze();
        for (const v of violations) {
          failures.push(`${where}: axe ${v.id}, ${v.help} (${v.nodes.length} places, ${v.helpUrl})`);
        }
      }
    }
    await context.close();
  }
} finally {
  await browser.close();
}

for (const failure of failures) console.log(failure);
if (failures.length > 0) {
  console.log(`${failures.length} browser problems found on ${pages.length} pages. Readers on those widths meet them. Fix each one in the crate that renders it, then run scripts/demo-check.sh again.`);
  process.exit(1);
}
console.log(`All ${pages.length} pages pass axe and the layout checks at six widths, with scripts on and off.`);
