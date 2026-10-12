// Spydus OPAC "ALLWRKENQ" results-page → JSON (also yields the setId of edition-level ALLENQ pages).
//
// Engine: Bun's native HTMLRewriter (lol-html) in a single streaming pass.
// There is no DOM and no random access — handlers fire in document order — so we
// keep a `current` record pointer that descendant handlers write into, opened on
// each `<fieldset class="card card-list">` and sealed on its end tag.

import { collapse, decode, toInt, absolute } from "./decode";
import type { SearchRecord, SearchResults, RecordFormat } from "../types";

interface FormatDraft {
  code: string;
  label: string;
  reserveUrl: string | null;
  availabilityUrl: string | null;
}

interface Draft {
  rank: number | null;
  workIrn: string | null;
  titleBuf: string;
  detailUrl: string | null;
  authorUrl: string | null;
  detailLines: string[];
  /** Per detail line: did it contain an author-search (AUTHENQ) link? */
  detailLinked: boolean[];
  lineBuf: string | null;
  lineLinked: boolean;
  summaryBuf: string;
  isbn: string | null;
  coverUrls: string[];
  formats: Map<string, FormatDraft>;
  fmtLabelCode: string | null;
  fmtLabelBuf: string;
}

// Relator terms (RDA) that end a creator line ("Rowling, J. K., author"), optionally qualified
// ("film director", "executive producer", "co-author"). Deliberately no publisher / distributor /
// issuing body: an imprint is never an author.
const ROLE_WORDS =
  "author|illustrator|editor|translator|adapter|artist|writer|narrator|composer|contributor|creator|" +
  "director|producer|screenwriter|performer|actor|photographer|compiler|cartographer|lyricist|" +
  "presenter|speaker|interviewer|interviewee|host|librettist|arranger|conductor|singer|musician|" +
  "instrumentalist|animator|cinematographer|choreographer|abridger|reader";
const ROLE = `(?:(?:film|television|executive|voice|cover|joint)\\s+)?(?:co-?)?(?:${ROLE_WORDS})`;
// ", <role>[, <role>…]" at the end of the line, or a bare space after life dates
// ("Rowling, J. K., 1965- author."). Group 1 = those life dates (kept in the author, like
// "Haig, Matt, 1975-"); group 2 = the first role.
const AUTHOR_ROLE = new RegExp(
  `(?:,\\s*|(\\d{4}-(?:\\d{4})?\\.?)\\s+)(${ROLE})s?(?:\\s*,\\s*${ROLE}s?)*\\.?$`,
  "i",
);
// Role-less personal name with life dates: "Haig, Matt, 1975-" / "Rowling, J. K. 1965-".
const NAME_WITH_DATES = /^[^\d,:;\/()[\]]+,[^\d,:;\/()[\]]+(?:,\s*|\s+)\d{4}-(?:\d{4})?\.?$/;
const YEAR = /^\d{4}(\s*[-–]\s*\d{4})?$/;

function newDraft(): Draft {
  return {
    rank: null,
    workIrn: null,
    titleBuf: "",
    detailUrl: null,
    authorUrl: null,
    detailLines: [],
    detailLinked: [],
    lineBuf: null,
    lineLinked: false,
    summaryBuf: "",
    isbn: null,
    coverUrls: [],
    formats: new Map(),
    fmtLabelCode: null,
    fmtLabelBuf: "",
  };
}

function fmtOf(d: Draft, code: string): FormatDraft {
  let f = d.formats.get(code);
  if (!f) {
    f = { code, label: "", reserveUrl: null, availabilityUrl: null };
    d.formats.set(code, f);
  }
  return f;
}

/** Parse `id="BK-pane-15974354-url"` / `id="BK-tab-15974354"` → format code. */
function codeFromId(id: string | null): string | null {
  if (!id) return null;
  const m = id.match(/^([A-Z0-9]+)-(?:tab|pane)-/i);
  return m ? m[1].toUpperCase() : null;
}

/** Parse `...?...&RECFMT=BK` → format code. */
function codeFromQuery(href: string | null): string | null {
  if (!href) return null;
  const m = decode(href).match(/[?&]RECFMT=([A-Z0-9]+)/i);
  return m ? m[1].toUpperCase() : null;
}

function parseCovers(dataImgurls: string | null): { covers: string[]; isbn: string | null } {
  if (!dataImgurls) return { covers: [], isbn: null };
  const parts = decode(dataImgurls)
    .split("**")
    .map((s) => s.trim())
    .filter(Boolean)
    // drop Syndetics placeholders with no ISBN (…index.aspx?isbn=/mc.gif)
    .filter((u) => !/syndetics\.com\/index\.aspx\?isbn=\/mc\.gif/i.test(u));
  const covers = [...new Set(parts)];
  let isbn: string | null = null;
  for (const u of parts) {
    const m = u.match(/isbn=(\d{9,13}[Xx]?)/);
    if (m) {
      isbn = m[1];
      break;
    }
  }
  return { covers, isbn };
}

function finalize(d: Draft, origin: string): SearchRecord {
  // Derive author / year / language from the detail lines.
  let author: string | null = null;
  let authorRole: string | null = null;
  let year: string | null = null;
  let language: string | null = null;

  let first = true;
  for (const [i, raw] of d.detailLines.entries()) {
    const line = collapse(raw);
    if (!line) continue;
    const isFirst = first;
    first = false;
    const role = line.match(AUTHOR_ROLE);
    if (role && author === null) {
      authorRole = role[2].toLowerCase();
      author = (line.slice(0, role.index) + (role[1] ?? "")).trim() || null;
      continue;
    }
    // Guarded fallback: a role-less first line is the author only with structural evidence —
    // an author-search link, or a personal name with life dates. A bare "Haig, Matt" without a
    // link is indistinguishable from a language list ("English, French") and stays unclassified.
    if (isFirst && !role && !YEAR.test(line) && (d.detailLinked[i] || NAME_WITH_DATES.test(line))) {
      author = line;
      continue;
    }
    if (year === null && YEAR.test(line)) {
      year = line; // single year "2025" or a range "2022 - 2023"
      continue;
    }
    if (language === null && /^[A-Z][a-z]+(?:,\s*[A-Z][a-z]+)*$/.test(line) && !role) {
      language = line;
    }
  }

  const formats: RecordFormat[] = [...d.formats.values()].map((f) => ({
    code: f.code,
    label: collapse(f.label) || null,
    reserveUrl: absolute(origin, f.reserveUrl),
    availabilityUrl: absolute(origin, f.availabilityUrl),
  }));

  return {
    rank: d.rank,
    workIrn: d.workIrn,
    title: collapse(d.titleBuf) || null,
    detailUrl: absolute(origin, d.detailUrl),
    author,
    authorRole,
    authorUrl: absolute(origin, d.authorUrl),
    year,
    language,
    summary: collapse(d.summaryBuf) || null,
    isbn: d.isbn,
    coverUrls: d.coverUrls,
    formats,
  };
}

export interface ParseSearchOptions {
  /** Origin to absolutise links against. Default: canning. */
  origin?: string;
  /** The original user query, echoed back into the result. */
  query?: string;
}

export function parseSearchResults(html: string, opts: ParseSearchOptions = {}): SearchResults {
  const origin = opts.origin ?? "https://canning.spydus.com";
  const records: SearchRecord[] = [];
  let current: Draft | null = null;

  let setId: string | null = null;
  let headerBuf = "";
  let nextPageUrl: string | null = null;
  let prevPageUrl: string | null = null;

  const rw = new HTMLRewriter()
    // ---- pagination: Next / Previous links ----
    .on(".result-pages-prvnxt .nxt a", {
      element(el) {
        if (!nextPageUrl) nextPageUrl = el.getAttribute("href");
      },
    })
    .on(".result-pages-prvnxt .prv a", {
      element(el) {
        if (!prevPageUrl) prevPageUrl = el.getAttribute("href");
      },
    })
    // ---- page-level: result count header ("Showing 1 - 10 of 46") ----
    .on(".result-header-brief", {
      text(t) {
        headerBuf += t.text;
      },
    })
    // ---- page-level: setId from the first ALLWRKENQ (work) or ALLENQ (edition) link ----
    .on('a[href*="/OPAC/ALL"]', {
      element(el) {
        if (setId) return;
        const m = decode(el.getAttribute("href") ?? "").match(/\/OPAC\/ALL(?:WRK)?ENQ\/(\d+)/);
        if (m) setId = m[1];
      },
    })
    // ---- record boundary ----
    .on("fieldset.card-list", {
      element(el) {
        const d = newDraft();
        current = d;
        el.onEndTag(() => {
          records.push(finalize(d, origin));
          if (current === d) current = null;
        });
      },
    })
    .on("fieldset.card-list span.item-num", {
      text(t) {
        if (current && current.rank == null) current.rank = toInt(t.text);
      },
    })
    .on('fieldset.card-list input[name="SVL"]', {
      element(el) {
        if (current && !current.workIrn) current.workIrn = el.getAttribute("value");
      },
    })
    // ---- title ----
    .on("fieldset.card-list h3.card-title a", {
      element(el) {
        if (current && !current.detailUrl) current.detailUrl = el.getAttribute("href");
      },
      text(t) {
        if (current) current.titleBuf += t.text;
      },
    })
    // ---- author / year / language lines ----
    .on("fieldset.card-list div.recdetails a", {
      element(el) {
        if (current && !current.authorUrl) current.authorUrl = el.getAttribute("href");
      },
    })
    .on("fieldset.card-list div.recdetails > span.d-block", {
      element(el) {
        if (current) {
          current.lineBuf = "";
          current.lineLinked = false;
        }
        el.onEndTag(() => {
          if (current && current.lineBuf != null) {
            current.detailLines.push(current.lineBuf);
            current.detailLinked.push(current.lineLinked);
            current.lineBuf = null;
          }
        });
      },
      text(t) {
        if (current && current.lineBuf != null) current.lineBuf += t.text;
      },
    })
    .on('fieldset.card-list div.recdetails > span.d-block a[href*="/AUTHENQ/"]', {
      element() {
        if (current && current.lineBuf != null) current.lineLinked = true;
      },
    })
    // ---- summary / blurb ----
    .on("fieldset.card-list div.summary .text-html", {
      text(t) {
        if (current) current.summaryBuf += t.text;
      },
    })
    // ---- cover image + ISBN ----
    .on("fieldset.card-list img.imgsc", {
      element(el) {
        if (!current) return;
        const { covers, isbn } = parseCovers(el.getAttribute("data-imgurls"));
        if (covers.length) current.coverUrls = covers;
        if (isbn) current.isbn = isbn;
      },
    })
    // ---- formats: tab labels ----
    .on("fieldset.card-list .card-body-tabs .nav-link", {
      element(el) {
        if (!current) return;
        const code = codeFromId(el.getAttribute("id"));
        current.fmtLabelCode = code;
        current.fmtLabelBuf = "";
        el.onEndTag(() => {
          if (current && current.fmtLabelCode) {
            fmtOf(current, current.fmtLabelCode).label = current.fmtLabelBuf;
            current.fmtLabelCode = null;
          }
        });
      },
      text(t) {
        if (current && current.fmtLabelCode != null) current.fmtLabelBuf += t.text;
      },
    })
    // ---- formats: reservation link ----
    .on('fieldset.card-list .tab-pane a[href*="/PGM/OPAC/CCOPT/"]', {
      element(el) {
        if (!current) return;
        const href = el.getAttribute("href");
        const code = codeFromQuery(href);
        if (code) fmtOf(current, code).reserveUrl = href;
      },
    })
    // ---- formats: lazy availability fragment url ----
    .on("fieldset.card-list .tab-pane-url", {
      element(el) {
        if (!current) return;
        const code = codeFromId(el.getAttribute("id"));
        const href = el.getAttribute("data-tab-href");
        if (code && href) fmtOf(current, code).availabilityUrl = href;
      },
    });

  rw.transform(html);

  const showingMatch = collapse(headerBuf).match(/Showing\s+([\d,]+)\s*-\s*([\d,]+)\s+of\s+([\d,]+)/i);
  const showing = showingMatch
    ? { from: toInt(showingMatch[1])!, to: toInt(showingMatch[2])! }
    : null;
  const totalResults = showingMatch ? toInt(showingMatch[3]) : records.length || null;

  return {
    query: opts.query ?? null,
    setId,
    totalResults,
    showing,
    nextPageUrl: absolute(origin, nextPageUrl),
    prevPageUrl: absolute(origin, prevPageUrl),
    records,
  };
}
