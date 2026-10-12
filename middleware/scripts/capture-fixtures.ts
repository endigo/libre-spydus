// Capture PUBLIC OPAC fixtures for the Rust core's golden tests.
//   bun middleware/scripts/capture-fixtures.ts           Canning set (search-midnight, bk-recfmt, …)
//   bun middleware/scripts/capture-fixtures.ts --multi   multi-library set → fixtures/multi/
//
// Safety (docs/rust-core-port.md §3): public GETs only, sequential, no cookies or
// session (the PQID cookie the results page sets is never sent back), at most
// MAX_REQUESTS upstream requests per run. Writes into core/tests/fixtures/.
import { fetchSearch, fetchFragment, ORIGIN } from "../src/fetch/spydus";
import { parseSearchResults } from "../src/parse/search-results";
import { bibLinkFromFragment, parseRecordDetail } from "../src/parse/record";

const QUERY = "the midnight library";
const MAX_REQUESTS = 20;
const OUT = new URL("../../core/tests/fixtures/", import.meta.url);

// Wave 2 (contract §10): one work-level results page per library, fetched by building the
// OPAC search URL directly (the discover API is not universal). Name = first host label.
const MULTI_QUERY = "harry potter";
const MULTI_ORIGINS = [
  "https://camden.spydus.com",
  "https://cumberlandlibraries.spydus.com",
  "https://esperance.spydus.com",
  "https://porirua.spydus.co.nz",
  "https://hutt.spydus.co.nz",
  "https://niagara.spydus.com",
];

let requests = 0;
function budget(n: number) {
  requests += n;
  if (requests > MAX_REQUESTS) throw new Error(`request budget exceeded (${requests} > ${MAX_REQUESTS})`);
}

function guard(url: string, origin = ORIGIN): string {
  if (!url.startsWith(origin + "/")) throw new Error(`refusing non-origin URL: ${url}`);
  return url;
}

async function get(url: string, origin = ORIGIN): Promise<string> {
  budget(1);
  return fetchFragment(guard(url, origin)); // no cookie argument → cookieless GET
}

async function save(name: string, html: string) {
  await Bun.write(new URL(name, OUT), html);
  console.log(`  wrote ${name.padEnd(40)} ${html.length.toLocaleString()} chars`);
}

/** `{origin}/cgi-bin/spydus.exe/ENQ/OPAC/ALLWRKENQ?ENTRY=…&GQ=…` (form-encoded, spaces '+'). */
function allWrkEnqUrl(origin: string, query: string): string {
  const params = new URLSearchParams([
    ["ENTRY", query],
    ["ENTRY_NAME", "BS"],
    ["ENTRY_TYPE", "K"],
    ["SORTS", "SQL_REL_GENWRK"],
    ["GQ", query],
  ]);
  return `${origin}/cgi-bin/spydus.exe/ENQ/OPAC/ALLWRKENQ?${params}`;
}

if (process.argv.includes("--multi")) {
  const slug = MULTI_QUERY.replace(/\s+/g, "-");
  let failed = 0;
  for (const origin of MULTI_ORIGINS) {
    const label = new URL(origin).hostname.split(".")[0];
    try {
      const html = await get(allWrkEnqUrl(origin, MULTI_QUERY), origin);
      await save(`multi/${label}-${slug}.html`, html);
    } catch (e) {
      failed++;
      console.error(`  ${label}: ${(e as Error).message}`);
    }
  }
  console.log(`done: ${requests} upstream GETs, ${failed} failed`);
  process.exit(failed ? 1 : 0);
}

// 1. Search results (discover/search resolve + results page = 2 GETs).
budget(2);
const page = await fetchSearch(QUERY);
await save("search-midnight.html", page.html);

// 2. BK-format availability fragment of the first record that has one.
const results = parseSearchResults(page.html, { query: QUERY });
const bk = results.records
  .flatMap((r) => r.formats)
  .find((f) => f.code === "BK" && f.availabilityUrl);
if (!bk?.availabilityUrl) throw new Error("no BK availabilityUrl in search results");
const recfmt = await get(bk.availabilityUrl);
await save("bk-recfmt.html", recfmt);

// 3. The bib page that fragment links to.
const link = bibLinkFromFragment(recfmt, ORIGIN);
if (!link) throw new Error("no bib link in BK fragment");
const bib = await get(link.bibUrl);
await save("record-bib.html", bib);

// 4. One related-titles panel: SERIES, else SIMILAR, else AUTHOR.
const rel = parseRecordDetail(bib, ORIGIN, link.bibIrn).related;
const panel = rel.series ?? rel.similar ?? rel.author;
if (!panel) throw new Error("record-bib has no SERIES/SIMILAR/AUTHOR panel");
const related = await get(panel);
await save("related.html", related);

console.log(`done: ${requests} upstream GETs; bibIrn=${link.bibIrn}; panel=${panel.slice(ORIGIN.length)}`);
