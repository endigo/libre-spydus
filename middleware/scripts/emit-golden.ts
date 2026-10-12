// Run the TS parsers (the parity oracle) over the Rust core's fixtures and write
// pretty JSON goldens + MANIFEST.md.
//   bun middleware/scripts/emit-golden.ts
//
// Deterministic: re-running over the same fixtures rewrites identical files. Private rows
// (personal account data) are skipped when core/tests/fixtures/private/ is absent; their
// manifest rows are always listed by name only. Never prints fixture or golden contents.
import { existsSync, mkdirSync } from "node:fs";
import { ORIGIN } from "../src/fetch/spydus";
import { parseSearchResults } from "../src/parse/search-results";
import { parseAvailability } from "../src/parse/availability";
import { bibLinkFromFragment, parseRecordDetail, parseBriefCards } from "../src/parse/record";
import {
  parseReserveForm,
  toReserveOptions,
  parseEditForm,
  toEditOptions,
  parseManageResult,
  parseReserveResult,
} from "../src/parse/reserve";
import { parseAccountLinks, parsePatronItems } from "../src/parse/account";

const TESTS = new URL("../../core/tests/", import.meta.url);
const FIXTURES = new URL("fixtures/", TESTS);
const GOLDEN = new URL("golden/", TESTS);

interface Row {
  /** Path under core/tests/fixtures/. */
  fixture: string;
  /** TS function name (as imported above). */
  ts: string;
  /** Exact TS arguments, `html` = the fixture text. */
  args: string;
  /** Path under core/tests/golden/. */
  golden: string;
  /** Rust call; `html` = `common::fixture("<fixture>")`, `ORIGIN` = `config::ORIGIN`. */
  rust: string;
  /** What the row exercises (manifest only). */
  note?: string;
  run: (html: string) => unknown;
}

const read = (rel: string) => Bun.file(new URL(rel, FIXTURES)).text();
const q = (s: string) => JSON.stringify(s);

// record-bib.json is parsed with the bib IRN its BK fragment links to (as /record does).
const bkLink = bibLinkFromFragment(await read("bk-recfmt.html"), ORIGIN);
if (!bkLink) throw new Error("bk-recfmt.html has no bib link");
const BIB_IRN = bkLink.bibIrn;

const BSC_QUERY = "The baby-sitters club graphic novel";
const MIDNIGHT_QUERY = "the midnight library";
// Wave 2 multi-library fixtures (`capture-fixtures.ts --multi`), each parsed with its own origin.
const MULTI_QUERY = "harry potter";
const MULTI_ORIGINS = [
  "https://camden.spydus.com",
  "https://cumberlandlibraries.spydus.com",
  "https://esperance.spydus.com",
  "https://porirua.spydus.co.nz",
  "https://hutt.spydus.co.nz",
  "https://niagara.spydus.com",
];
// Phase 2 (P1): each library's genuine empty result for a nonsense keyword.
const NO_RESULTS_QUERY = "zzqxjv";
const NO_RESULTS_ORIGINS = [
  "https://canning.spydus.com",
  "https://cumberlandlibraries.spydus.com",
  "https://hutt.spydus.co.nz",
  "https://niagara.spydus.com",
  "https://mybcls.spydus.com",
];
// Canning's answer to the keyword "*": a different alert on the same "Error result" page.
const EMPTY_SEARCH_QUERY = "*";

const ROWS: Row[] = [
  // ---- public: search + availability (task B) ----
  {
    fixture: "search.html", golden: "search.json",
    ts: "parseSearchResults", args: `html, { query: ${q(BSC_QUERY)} }`,
    rust: `parse::search::parse_search_results(&html, ORIGIN, Some(${q(BSC_QUERY)}))`,
    run: (h) => parseSearchResults(h, { query: BSC_QUERY }),
  },
  {
    fixture: "search-midnight.html", golden: "search-midnight.json",
    ts: "parseSearchResults", args: `html, { query: ${q(MIDNIGHT_QUERY)} }`,
    rust: `parse::search::parse_search_results(&html, ORIGIN, Some(${q(MIDNIGHT_QUERY)}))`,
    run: (h) => parseSearchResults(h, { query: MIDNIGHT_QUERY }),
  },
  {
    fixture: "availability.html", golden: "availability.json",
    ts: "parseAvailability", args: "html",
    rust: "parse::availability::parse_availability(&html, ORIGIN)",
    run: (h) => parseAvailability(h),
  },
  {
    fixture: "bk-recfmt.html", golden: "availability-midnight.json",
    ts: "parseAvailability", args: "html",
    rust: "parse::availability::parse_availability(&html, ORIGIN)",
    run: (h) => parseAvailability(h),
  },
  // ---- public: other libraries' work-level results (Wave 2, G1) ----
  ...MULTI_ORIGINS.map((origin): Row => {
    const stem = `multi/${new URL(origin).hostname.split(".")[0]}-${MULTI_QUERY.replace(/\s+/g, "-")}`;
    return {
      fixture: `${stem}.html`, golden: `${stem}.json`,
      ts: "parseSearchResults", args: `html, { origin: ${q(origin)}, query: ${q(MULTI_QUERY)} }`,
      rust: `parse::search::parse_search_results(&html, ${q(origin)}, Some(${q(MULTI_QUERY)}))`,
      run: (h) => parseSearchResults(h, { origin, query: MULTI_QUERY }),
    };
  }),
  // ---- public: the OPAC's "did not find any records" page (Phase 2, P1) ----
  ...NO_RESULTS_ORIGINS.map((origin): Row => {
    const stem = `multi/${new URL(origin).hostname.split(".")[0]}-no-results`;
    return {
      fixture: `${stem}.html`, golden: `${stem}.json`,
      ts: "parseSearchResults", args: `html, { origin: ${q(origin)}, query: ${q(NO_RESULTS_QUERY)} }`,
      rust: `parse::search::parse_search_results(&html, ${q(origin)}, Some(${q(NO_RESULTS_QUERY)}))`,
      note: "genuine empty result → noResults: true, no records",
      run: (h) => parseSearchResults(h, { origin, query: NO_RESULTS_QUERY }),
    };
  }),
  {
    fixture: "multi/canning-empty-search.html", golden: "multi/canning-empty-search.json",
    ts: "parseSearchResults", args: `html, { query: ${q(EMPTY_SEARCH_QUERY)} }`,
    rust: `parse::search::parse_search_results(&html, ORIGIN, Some(${q(EMPTY_SEARCH_QUERY)}))`,
    note: "other error page (\"Please enter something in the search form.\") → noResults stays false, no records",
    run: (h) => parseSearchResults(h, { query: EMPTY_SEARCH_QUERY }),
  },
  // ---- synthetic: search edge cases (Wave 2, G1) ----
  {
    fixture: "synthetic/search-author-lines.html", golden: "synthetic/search-author-lines.json",
    ts: "parseSearchResults", args: "html, {}",
    rust: "parse::search::parse_search_results(&html, ORIGIN, None)",
    note: "author rules: life dates before role, qualified role, role chain, new roles, role-less linked / dated name, ambiguous name, publisher/imprint/edition/format never author",
    run: (h) => parseSearchResults(h, {}),
  },
  {
    fixture: "synthetic/search-allenq.html", golden: "synthetic/search-allenq.json",
    ts: "parseSearchResults", args: "html, {}",
    rust: "parse::search::parse_search_results(&html, ORIGIN, None)",
    note: "setId from an edition-level ALLENQ link (set-less ALLENQ link skipped)",
    run: (h) => parseSearchResults(h, {}),
  },
  // ---- public: record + related (task C) ----
  {
    fixture: "availability.html", golden: "biblink-bsc.json",
    ts: "bibLinkFromFragment", args: "html, ORIGIN",
    rust: "parse::record::bib_link_from_fragment(&html, ORIGIN)",
    run: (h) => bibLinkFromFragment(h, ORIGIN),
  },
  {
    fixture: "bk-recfmt.html", golden: "biblink.json",
    ts: "bibLinkFromFragment", args: "html, ORIGIN",
    rust: "parse::record::bib_link_from_fragment(&html, ORIGIN)",
    run: (h) => bibLinkFromFragment(h, ORIGIN),
  },
  {
    fixture: "record.html", golden: "record.json",
    ts: "parseRecordDetail", args: "html, ORIGIN, null",
    rust: "parse::record::parse_record_detail(&html, ORIGIN, None)",
    run: (h) => parseRecordDetail(h, ORIGIN, null),
  },
  {
    fixture: "record-bib.html", golden: "record-bib.json",
    ts: "parseRecordDetail", args: `html, ORIGIN, ${q(BIB_IRN)}`,
    rust: `parse::record::parse_record_detail(&html, ORIGIN, Some(${q(BIB_IRN)}.into()))`,
    note: "bibIrn = biblink.json bibIrn",
    run: (h) => parseRecordDetail(h, ORIGIN, BIB_IRN),
  },
  {
    fixture: "related.html", golden: "related.json",
    ts: "parseBriefCards", args: "html, ORIGIN",
    rust: "parse::record::parse_brief_cards(&html, ORIGIN)",
    run: (h) => parseBriefCards(h, ORIGIN),
  },
  // ---- synthetic: reserve forms + results (task D) ----
  ...manageRows("manage-cancel-success", ["cancel"], "success wording; mainForm kept so wording decides"),
  ...manageRows("manage-edit-success", ["edit"], "success wording; mainForm kept so wording decides"),
  ...manageRows("manage-validation-error", ["edit"], "populated formValidationMsg (trimmed)"),
  ...manageRows("manage-cannot-cancel", ["cancel", "edit"], "cancel: refusal sentence wins over success wording (Wave 2); edit: generic failure match; placeholder `Error` msg ignored"),
  ...manageRows("manage-cancel-refused", ["cancel"], "refusal in an alert, no mainForm left: still a failure, with the sentence (entity-decoded, inline tags dropped)"),
  ...manageRows("manage-cancel-success-help", ["cancel"], "list page with help text (negations near other verbs, refusal text only in script/style) → success"),
  ...manageRows("manage-still-on-form", ["cancel", "edit"], "still on mainForm, no wording → default failure messages"),
  ...manageRows("manage-left-wizard", ["cancel", "edit"], "no mainForm left → success"),
  ...reserveResultRow("reserve-placed", "success wording; mainForm kept so wording decides"),
  ...reserveResultRow("reserve-already-reserved", "failure wording"),
  ...reserveResultRow("reserve-still-on-form", "placeholder `Error` msg ignored; still on mainForm"),
  ...reserveResultRow("reserve-no-form", "no form, no wording → placed"),
  ...reserveResultRow("reserve-validation-error", "populated formValidationMsg"),
  {
    fixture: "synthetic/reserve-form.html", golden: "synthetic/reserve-form.json",
    ts: "parseReserveForm", args: "html, ORIGIN",
    rust: "parse::reserve::parse_reserve_form(&html, ORIGIN)",
    note: "every input/select/button branch; double-decoded action; decoy statics outside the form",
    run: (h) => parseReserveForm(h, ORIGIN),
  },
  {
    fixture: "synthetic/reserve-form.html", golden: "synthetic/reserve-options.json",
    ts: "toReserveOptions(parseReserveForm(…))", args: "html, ORIGIN",
    rust: "parse::reserve::to_reserve_options(parse::reserve::parse_reserve_form(&html, ORIGIN).as_ref())",
    run: (h) => toReserveOptions(parseReserveForm(h, ORIGIN)),
  },
  {
    fixture: "synthetic/edit-form.html", golden: "synthetic/edit-form.json",
    ts: "parseEditForm", args: "html, ORIGIN",
    rust: "parse::reserve::parse_edit_form(&html, ORIGIN)",
    note: "PICKUP choices (empty skipped, entity-decoded), selected-after-value, SDTE set, XDTE empty",
    run: (h) => parseEditForm(h, ORIGIN),
  },
  {
    fixture: "synthetic/edit-form.html", golden: "synthetic/edit-options.json",
    ts: "toEditOptions(parseEditForm(…))", args: "html, ORIGIN",
    rust: "parse::reserve::to_edit_options(parse::reserve::parse_edit_form(&html, ORIGIN).as_ref())",
    run: (h) => toEditOptions(parseEditForm(h, ORIGIN)),
  },
  {
    fixture: "synthetic/reserve-no-form.html", golden: "synthetic/reserve-form-none.json",
    ts: "parseReserveForm", args: "html, ORIGIN",
    rust: "parse::reserve::parse_reserve_form(&html, ORIGIN)",
    note: "no mainForm → null",
    run: (h) => parseReserveForm(h, ORIGIN),
  },
  {
    fixture: "synthetic/reserve-no-form.html", golden: "synthetic/reserve-options-none.json",
    ts: "toReserveOptions(parseReserveForm(…))", args: "html, ORIGIN",
    rust: "parse::reserve::to_reserve_options(parse::reserve::parse_reserve_form(&html, ORIGIN).as_ref())",
    run: (h) => toReserveOptions(parseReserveForm(h, ORIGIN)),
  },
  {
    fixture: "synthetic/reserve-no-form.html", golden: "synthetic/edit-options-none.json",
    ts: "toEditOptions(parseEditForm(…))", args: "html, ORIGIN",
    rust: "parse::reserve::to_edit_options(parse::reserve::parse_edit_form(&html, ORIGIN).as_ref())",
    run: (h) => toEditOptions(parseEditForm(h, ORIGIN)),
  },
  // ---- synthetic: account (task D) ----
  {
    fixture: "synthetic/account-links.html", golden: "synthetic/account-links.json",
    ts: "parseAccountLinks", args: "html, ORIGIN",
    rust: "parse::account::parse_account_links(&html, ORIGIN)",
    note: "FMT filter, first-wins, non-OPAC link ignored, navbar name",
    run: (h) => parseAccountLinks(h, ORIGIN),
  },
  {
    fixture: "synthetic/patron-items.html", golden: "synthetic/patron-items.json",
    ts: "parsePatronItems", args: "html, ORIGIN",
    rust: "parse::account::parse_patron_items(&html, ORIGIN)",
    note: "header row + blank-title row dropped, IRN( bibIrn, covers, fields, deduped actions, total with comma",
    run: (h) => parsePatronItems(h, ORIGIN),
  },
  // ---- private: personal account data (task D); skipped when absent ----
  privateRow("account-details-opac.html", "account-links.json", "parseAccountLinks", "parse::account::parse_account_links(&html, ORIGIN)", (h) => parseAccountLinks(h, ORIGIN)),
  privateRow("loans-1.html", "loans-1.json", "parsePatronItems", "parse::account::parse_patron_items(&html, ORIGIN)", (h) => parsePatronItems(h, ORIGIN)),
  privateRow("reservations-2.html", "reservations-2.json", "parsePatronItems", "parse::account::parse_patron_items(&html, ORIGIN)", (h) => parsePatronItems(h, ORIGIN)),
  privateRow("reservations-5.html", "reservations-5.json", "parsePatronItems", "parse::account::parse_patron_items(&html, ORIGIN)", (h) => parsePatronItems(h, ORIGIN)),
  privateRow("reserve-form.html", "reserve-form.json", "parseReserveForm", "parse::reserve::parse_reserve_form(&html, ORIGIN)", (h) => parseReserveForm(h, ORIGIN)),
  privateRow("reserve-form.html", "reserve-options.json", "toReserveOptions(parseReserveForm(…))", "parse::reserve::to_reserve_options(parse::reserve::parse_reserve_form(&html, ORIGIN).as_ref())", (h) => toReserveOptions(parseReserveForm(h, ORIGIN))),
  privateRow("cancel-confirm.html", "cancel-form.json", "parseReserveForm", "parse::reserve::parse_reserve_form(&html, ORIGIN)", (h) => parseReserveForm(h, ORIGIN)),
  privateRow("edit-form.html", "edit-form.json", "parseEditForm", "parse::reserve::parse_edit_form(&html, ORIGIN)", (h) => parseEditForm(h, ORIGIN)),
  privateRow("edit-form.html", "edit-options.json", "toEditOptions(parseEditForm(…))", "parse::reserve::to_edit_options(parse::reserve::parse_edit_form(&html, ORIGIN).as_ref())", (h) => toEditOptions(parseEditForm(h, ORIGIN))),
  // A successful cancel lands on these list pages: the refusal rule must not fire on them.
  privateManageRow("reservations-2.html", "reservations-2.cancel.json"),
  privateManageRow("reservations-5.html", "reservations-5.cancel.json"),
  privateManageRow("loans-1.html", "loans-1.cancel.json"),
];

function manageRows(stem: string, kinds: ("cancel" | "edit")[], note: string): Row[] {
  return kinds.map((kind) => ({
    fixture: `synthetic/${stem}.html`, golden: `synthetic/${stem}.${kind}.json`,
    ts: "parseManageResult", args: `html, ${q(kind)}`,
    rust: `parse::reserve::parse_manage_result(&html, parse::reserve::ManageKind::${kind === "cancel" ? "Cancel" : "Edit"})`,
    note,
    run: (h: string) => parseManageResult(h, kind),
  }));
}

function reserveResultRow(stem: string, note: string): Row[] {
  return [{
    fixture: `synthetic/${stem}.html`, golden: `synthetic/${stem}.json`,
    ts: "parseReserveResult", args: "html",
    rust: "parse::reserve::parse_reserve_result(&html)",
    note,
    run: (h: string) => parseReserveResult(h),
  }];
}

function privateRow(fixture: string, golden: string, ts: string, rust: string, run: (h: string) => unknown): Row {
  const args = "html, ORIGIN";
  return { fixture: `private/${fixture}`, golden: `private/${golden}`, ts, args, rust, run };
}

function privateManageRow(fixture: string, golden: string): Row {
  return {
    fixture: `private/${fixture}`, golden: `private/${golden}`,
    ts: "parseManageResult", args: `html, "cancel"`,
    rust: "parse::reserve::parse_manage_result(&html, parse::reserve::ManageKind::Cancel)",
    note: "real list page = post-cancel landing page → success; refusal rule must not fire",
    run: (h) => parseManageResult(h, "cancel"),
  };
}

// ---- emit goldens ----
const privateAvailable = existsSync(new URL("private/", FIXTURES));
for (const dir of ["", "synthetic/", "private/", "multi/"]) mkdirSync(new URL(dir, GOLDEN), { recursive: true });

let written = 0;
let skipped = 0;
for (const row of ROWS) {
  if (row.fixture.startsWith("private/") && !privateAvailable) {
    skipped++;
    continue;
  }
  const value = row.run(await read(row.fixture));
  const json = JSON.stringify(value, null, 2);
  if (json === undefined) throw new Error(`${row.golden}: parser returned undefined`);
  await Bun.write(new URL(row.golden, GOLDEN), json + "\n");
  written++;
}

// ---- MANIFEST.md (always lists every row; private rows are names only) ----
const cell = (s: string) => s.replaceAll("|", "\\|");
const table = ROWS.map(
  (r) =>
    `| \`${r.fixture}\` | \`${r.ts}\` | \`${cell(r.args)}\` | \`${r.golden}\` | \`${cell(r.rust)}\` | ${cell(r.note ?? "")} |`,
).join("\n");

const manifest = `# Golden manifest

Generated by \`bun middleware/scripts/emit-golden.ts\`. Do not edit by hand. Re-run it after
changing a fixture or a TS parser. The goldens are the TS parsers' output (the parity oracle).

Conventions for the Rust column: \`html\` = \`common::fixture("<fixture>")\`,
\`ORIGIN\` = \`librespydus_core::config::ORIGIN\` (the TS default origin), and paths are relative to
\`librespydus_core\`. Compare with \`common::assert_golden(&actual, "<golden>")\`. Rows under
\`private/\` hold personal account data. Tests must skip when \`common::private_available()\` is false,
and must never print the contents.

Fixture provenance: \`search.html\` = middleware/fixtures/search-allwrk.html; \`availability.html\` =
middleware/fixtures/record-recfmt.html; \`record.html\` = a guest (logged-out) BIBENQ capture;
\`encrypt.js\` = the OPAC RSA key script (crypto input, no golden). \`search-midnight.html\`,
\`bk-recfmt.html\`, \`record-bib.html\` and \`related.html\` came from
\`bun middleware/scripts/capture-fixtures.ts\` (cookieless public GETs). \`multi/*\` came from
\`bun middleware/scripts/capture-fixtures.ts --multi\`: one cookieless GET per library of
\`{origin}/cgi-bin/spydus.exe/ENQ/OPAC/ALLWRKENQ?ENTRY=harry+potter&ENTRY_NAME=BS&ENTRY_TYPE=K&SORTS=SQL_REL_GENWRK&GQ=harry+potter\`;
each is parsed with that library's origin. \`multi/*-no-results.html\` (2026-10-12) are the same
cookieless GET with the core's User-Agent for the nonsense keyword \`zzqxjv\` (\`ENTRY=zzqxjv…GQ=zzqxjv\`),
and \`multi/canning-empty-search.html\` is that GET on Canning for the keyword \`*\` (\`ENTRY=*…GQ=*\`).
\`synthetic/*\` is hand-written.

| fixture | TS function | TS args | golden | Rust call | exercises |
|---|---|---|---|---|---|
${table}

## parseCovers copies

\`search-results.ts:77-95\`, \`record.ts:9-25\` and \`account.ts:100-116\` agree. All three use the same
filters, the same order-preserving de-duplication (\`[...new Set(parts)]\`) and the same first
\`isbn=(\\d{9,13}[Xx]?)\` match over the pre-dedupe parts. The only difference is statement order
(search computes \`covers\` before \`isbn\`), which does not change the output. One Rust
\`parse::decode::parse_covers\` serves all three. What differs is how callers use it: search,
related cards and patron rows let the last \`img.imgsc\` with covers win, while \`parseRecordDetail\`
keeps the first.

## TS quirks to replicate (or knowingly not)

- \`decode\`: \`&nbsp;\` → U+0020 (a plain space, not U+00A0). Named lookup lower-cases the name
  (\`&AMP;\` → \`&\`). Decimal bodies use \`parseInt\` prefix semantics (\`&#12ab;\` → U+000C), and
  \`&#abc;\` (no decimal digits) is kept verbatim. Code points above U+10FFFF → \`""\`.
  *Not replicated:* a lone surrogate (\`&#xD800;\`) stays a lone UTF-16 unit in JS; Rust emits U+FFFD.
  \`&constructor;\` hits \`Object.prototype\` in JS and yields \`"function Object() { [native code] }"\`;
  Rust keeps it verbatim.
- \`toInt\`: integers beyond i64 become imprecise floats in TS; Rust returns \`None\`.
- JS regex classes: \`\\s\`/\`trim()\` include U+FEFF and exclude U+0085, unlike Rust \`\\s\`/\`str::trim\`.
  Use \`decode::JS_WS\` / \`decode::js_trim\`. JS \`\\d\`/\`\\w\` are ASCII-only. JS \`/i\` does not fold
  \`ſ\`/\`K\` (Kelvin) into ASCII, while Rust \`(?i)\` does.
- \`absolute\` decodes again. Values that were already decoded (reserve form \`action\`, account
  \`actHref\`) are therefore double-decoded (\`&amp;amp;\` → \`&\`).
- \`parseManageResult(kind="cancel")\` reports success for any page that says "cancel(l)ed" and
  "reservation|request|hold" anywhere, *unless* an explicit refusal comes first (below).
- \`formValidationMsg\`: the template's hidden placeholder text \`Error\` is ignored. It still counts
  as stripped text, so the generic \`(cannot|…|error)[^.]{0,80}\` failure match can pick it up.
- Form \`body\` order: JS objects put integer-like keys first. \`IndexMap\` keeps pure insertion order.
  Goldens compare as JSON objects, so the order is not asserted.
- \`parsePatronItems.total\` falls back to \`items.length\` (a number, possibly 0), never null.
- \`parseRecordDetail\`: \`related.versions\` reads the panel key \`version\` (\`urltabVERSION\`).

## Wave 2 deliberate divergences (made in the TS oracle and the Rust core alike)

- **Author detection** (\`search-results.ts\` \`finalize\`). A creator line ends in
  \`, <role>[, <role>…]\`, or in a bare space after life dates (\`Rowling, J. K., 1965- author.\`).
  The roles are the RDA relator terms in \`ROLE_WORDS\` (adds director, producer, screenwriter, performer,
  actor, photographer, compiler, cartographer, lyricist, presenter, speaker, interviewer, …), optionally
  qualified (\`film|television|executive|voice|cover|joint\`, \`co-\`). \`authorRole\` = the first role,
  lower-cased; life dates stay in \`author\`. There is no publisher, distributor or issuing-body role.
  Fallback: a role-less **first** line becomes the author (\`authorRole: null\`) only when it holds an
  \`a[href*="/AUTHENQ/"]\` link or is a personal name with life dates (\`Haig, Matt, 1975-\`), and is
  not a year. A bare, unlinked \`Haig, Matt\` still falls through to \`language\`, as before.
  Canning goldens (\`search.json\`, \`search-midnight.json\`) are byte-identical to before the change.
- **Cancel refusal** (\`reserve.ts\` \`cancelRefusal\`). For \`kind="cancel"\` only, after
  \`formValidationMsg\` and before the success wording, the page fails with the refusal's sentence.
  A refusal is a negation (cannot, can't, could not, couldn't, can no longer, unable to, not able to,
  not allowed to, not permitted to), then only be/now/currently/yet, then \`cancel…\`. Text source:
  \`<script>\`/\`<style>\` dropped, block tags become sentence edges (\`\\u0000\`), other tags become spaces,
  then entities are decoded. The sentence is up to 160 chars either side, ended by \`.\`, \`!\` or \`?\`,
  then whitespace-collapsed. Whole-page scope is safe on the real post-cancel landing pages (see the
  \`private/*.cancel.json\` rows). Residual risk: a library that prints help text such as "Reservations
  in transit cannot be cancelled" on its list page would report a successful cancel as refused.
- **setId** accepts edition-level \`/OPAC/ALLENQ/<n>\` as well as \`/OPAC/ALLWRKENQ/<n>\` (first
  matching \`a[href*="/OPAC/ALL"]\` wins).

## Phase 2 additions (made in the TS oracle and the Rust core alike)

- **noResults** (\`search-results.ts\`). The text of every \`.alert\` element is buffered, and
  \`noResults = records.length === 0 && /did not find any records/i.test(collapse(alertBuf))\`. The
  wording is the OPAC's own empty result, identical on the 9 AU/NZ/US libraries surveyed
  ("Sorry, your search for Anywhere: zzqxjv (Keywords) [in the local catalogue] did not find any
  records."). Any other page without records (another alert, a WAF page, a changed template) keeps
  \`noResults: false\`, so callers can tell "nothing found" from "page not understood". Every other
  search golden gains \`"noResults": false\` and is otherwise unchanged.
`;
await Bun.write(new URL("MANIFEST.md", GOLDEN), manifest);

console.log(
  `goldens: ${written} written, ${skipped} private skipped${privateAvailable ? "" : " (fixtures/private absent)"}; MANIFEST.md rows: ${ROWS.length}`,
);
