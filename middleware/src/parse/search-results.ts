// Spydus OPAC "ALLWRKENQ" results-page → JSON.
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
  lineBuf: string | null;
  summaryBuf: string;
  isbn: string | null;
  coverUrls: string[];
  formats: Map<string, FormatDraft>;
  fmtLabelCode: string | null;
  fmtLabelBuf: string;
}

const AUTHOR_ROLE = /,\s*(author|illustrator|editor|translator|adapter|artist|writer|narrator|composer|contributor|creator)s?\.?$/i;

function newDraft(): Draft {
  return {
    rank: null,
    workIrn: null,
    titleBuf: "",
    detailUrl: null,
    authorUrl: null,
    detailLines: [],
    lineBuf: null,
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

  for (const raw of d.detailLines) {
    const line = collapse(raw);
    if (!line) continue;
    const role = line.match(AUTHOR_ROLE);
    if (role && author === null) {
      authorRole = role[1].toLowerCase();
      author = line.replace(AUTHOR_ROLE, "").trim() || null;
      continue;
    }
    if (year === null && /^\d{4}(\s*[-–]\s*\d{4})?$/.test(line)) {
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
    // ---- page-level: setId from the first ALLWRKENQ link ----
    .on('a[href*="/OPAC/ALLWRKENQ/"]', {
      element(el) {
        if (setId) return;
        const m = decode(el.getAttribute("href") ?? "").match(/\/OPAC\/ALLWRKENQ\/(\d+)/);
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
        if (current) current.lineBuf = "";
        el.onEndTag(() => {
          if (current && current.lineBuf != null) {
            current.detailLines.push(current.lineBuf);
            current.lineBuf = null;
          }
        });
      },
      text(t) {
        if (current && current.lineBuf != null) current.lineBuf += t.text;
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
