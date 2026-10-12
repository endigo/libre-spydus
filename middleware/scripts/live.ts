// End-to-end live check + timing against canning.spydus.com.
//   bun run scripts/live.ts "The baby-sitters club graphic novel"
import { fetchSearch, fetchFragment, ORIGIN } from "../src/fetch/spydus";
import { parseSearchResults } from "../src/parse/search-results";
import { parseAvailability } from "../src/parse/availability";

const query = process.argv[2] ?? "The baby-sitters club graphic novel";

function ms(n: number) {
  return `${n.toFixed(1)}ms`;
}

const t0 = performance.now();
const page = await fetchSearch(query);
const tFetched = performance.now();

const tParse0 = performance.now();
const results = parseSearchResults(page.html, { query });
const tParse1 = performance.now();

console.log(`query        : ${query}`);
console.log(`results url  : ${page.url.slice(ORIGIN.length)}`);
console.log(`html bytes   : ${page.html.length.toLocaleString()}`);
console.log(`fetch        : ${ms(tFetched - t0)}  (network, origin render)`);
console.log(`PARSE engine : ${ms(tParse1 - tParse0)}  (HTMLRewriter → ${results.records.length} records)`);
console.log(`total / rec  : ${ms((tParse1 - tParse0) / Math.max(1, results.records.length))}`);
console.log(`setId=${results.setId} total=${results.totalResults} showing=${JSON.stringify(results.showing)}`);

const top = results.records[0];
console.log(`\ntop hit: #${top.rank} "${top.title}" — ${top.author} (${top.year}) [${top.formats.map((f) => f.code).join(", ")}]`);

// Follow the Book-format availability fragment to show live counts.
const bk = top.formats.find((f) => f.code === "BK") ?? top.formats[0];
if (bk?.availabilityUrl) {
  const fa0 = performance.now();
  const fragHtml = await fetchFragment(bk.availabilityUrl, page.cookie);
  const fa1 = performance.now();
  const avail = parseAvailability(fragHtml);
  const fa2 = performance.now();
  console.log(
    `\navailability (${bk.code}): copies=${avail.copies} reserved=${avail.reserved}` +
      `  fetch=${ms(fa1 - fa0)} parse=${ms(fa2 - fa1)}`,
  );
  console.log(`  bibRecordIrn=${avail.bibRecordIrn}  holdings=${avail.holdingsUrl?.slice(ORIGIN.length)}`);
}
