// Offline test harness: parse the captured fixtures and print JSON.
//   bun run scripts/test-parse.ts
import { parseSearchResults } from "../src/parse/search-results";
import { parseAvailability } from "../src/parse/availability";

const here = new URL("../fixtures/", import.meta.url);

const searchHtml = await Bun.file(new URL("search-allwrk.html", here)).text();
const recfmtHtml = await Bun.file(new URL("record-recfmt.html", here)).text();

const results = parseSearchResults(searchHtml, { query: "The baby-sitters club graphic novel" });

console.log("=".repeat(70));
console.log("SEARCH RESULTS");
console.log("=".repeat(70));
console.log(
  JSON.stringify(
    {
      query: results.query,
      setId: results.setId,
      totalResults: results.totalResults,
      showing: results.showing,
      recordCount: results.records.length,
    },
    null,
    2,
  ),
);

console.log("\n--- first 2 records ---");
console.log(JSON.stringify(results.records.slice(0, 2), null, 2));

// quick integrity summary across all records
const miss = (k: keyof (typeof results.records)[number]) =>
  results.records.filter((r) => r[k] == null || (Array.isArray(r[k]) && (r[k] as unknown[]).length === 0)).length;
console.log("\n--- field coverage (nulls/empties across all records) ---");
for (const k of ["workIrn", "title", "detailUrl", "author", "year", "isbn", "coverUrls", "formats"] as const) {
  console.log(`  ${k.padEnd(12)} missing in ${miss(k)}/${results.records.length}`);
}

console.log("\n" + "=".repeat(70));
console.log("AVAILABILITY FRAGMENT (RECFMT=BK)");
console.log("=".repeat(70));
console.log(JSON.stringify(parseAvailability(recfmtHtml), null, 2));
