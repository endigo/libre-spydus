// BIBENQ bib-record page → RecordDetail (the OPAC "Record details" block, the
// cover, the annotation, and the lazy related-title panel URLs), plus the brief
// cards inside a related-titles fragment (similar / by-author / in-series).

import { collapse, decode, absolute } from "./decode";
import type { DetailRow, DetailValue, RecordDetail, RelatedItem } from "../types";

// Reuse the cover/ISBN split used by the account parser (data-imgurls = "**"-joined).
function parseCovers(dataImgurls: string | null): { covers: string[]; isbn: string | null } {
  if (!dataImgurls) return { covers: [], isbn: null };
  const parts = decode(dataImgurls)
    .split("**")
    .map((s) => s.trim())
    .filter(Boolean)
    .filter((u) => !/syndetics\.com\/index\.aspx\?isbn=\/mc\.gif/i.test(u));
  let isbn: string | null = null;
  for (const u of parts) {
    const m = u.match(/isbn=(\d{9,13}[Xx]?)/);
    if (m) {
      isbn = m[1];
      break;
    }
  }
  return { covers: [...new Set(parts)], isbn };
}

/**
 * A format fragment (BK/EBK RECFMT) links to the concrete bib record it
 * resolves to: <a href="…/BIBENQ/{setId}?SETLVL=&IRN={bibIrn}">. Pull it out so
 * /record can fetch the rich page without guessing the bib IRN.
 */
export function bibLinkFromFragment(
  html: string,
  origin: string,
): { bibUrl: string; bibIrn: string } | null {
  const m = decode(html).match(/href="([^"]*\/BIBENQ\/\d+\?[^"]*IRN[=(](\d+)[^"]*)"/);
  if (!m) return null;
  const bibUrl = absolute(origin, m[1]);
  return bibUrl ? { bibUrl, bibIrn: m[2] } : null;
}

// ---------------------------------------------------------------------------
// Full bib record → RecordDetail
// ---------------------------------------------------------------------------

interface RowDraft {
  label: string;
  values: DetailValue[];
  valBuf: string | null;
  valHref: string | null;
}

export function parseRecordDetail(html: string, origin: string, bibIrn: string | null = null): RecordDetail {
  const details: DetailRow[] = [];
  let row: RowDraft | null = null;
  const covers: string[] = [];
  let isbnFromCover: string | null = null;
  const related: Record<string, string> = {};
  let summaryBuf: string | null = null;

  new HTMLRewriter()
    // One labelled row of the Record-details block.
    .on("#divtabRECDETAILS .row", {
      element(el) {
        const d: RowDraft = { label: "", values: [], valBuf: null, valHref: null };
        row = d;
        el.onEndTag(() => {
          if (d.valBuf != null) {
            const t = collapse(d.valBuf);
            if (t) d.values.push({ text: t, url: absolute(origin, d.valHref) });
          }
          const label = collapse(d.label).replace(/:\s*$/, "");
          if (label && d.values.length) details.push({ label, values: d.values });
          if (row === d) row = null;
        });
      },
    })
    .on("#divtabRECDETAILS .fd-caption", {
      text(t) {
        if (row) row.label += t.text;
      },
    })
    // Each value line is a <span class="d-block">; collect its text + first link.
    .on("#divtabRECDETAILS .col span.d-block", {
      element(el) {
        if (!row) return;
        if (row.valBuf != null) {
          const t = collapse(row.valBuf);
          if (t) row.values.push({ text: t, url: absolute(origin, row.valHref) });
        }
        row.valBuf = "";
        row.valHref = null;
        el.onEndTag(() => {
          if (row && row.valBuf != null) {
            const t = collapse(row.valBuf);
            if (t) row.values.push({ text: t, url: absolute(origin, row.valHref) });
            row.valBuf = null;
            row.valHref = null;
          }
        });
      },
      text(t) {
        if (row && row.valBuf != null) row.valBuf += t.text;
      },
    })
    .on("#divtabRECDETAILS .col span.d-block a[href]", {
      element(el) {
        if (row && row.valBuf != null && !row.valHref) row.valHref = el.getAttribute("href");
      },
    })
    // Main cover (first imgsc carrying real cover URLs).
    .on("img.imgsc", {
      element(el) {
        if (covers.length) return;
        const { covers: c, isbn } = parseCovers(el.getAttribute("data-imgurls"));
        if (c.length) {
          covers.push(...c);
          if (isbn) isbnFromCover = isbn;
        }
      },
    })
    // Lazy related-panel launch URLs: <a id="urltabSIMILAR" href="...">.
    .on("a[id]", {
      element(el) {
        const id = el.getAttribute("id") ?? "";
        if (id.startsWith("urltab")) {
          const name = id.slice(6).toLowerCase();
          const href = el.getAttribute("href");
          if (href && !related[name]) related[name] = href;
        }
      },
    })
    // Inline annotation/summary, when the page renders it (not lazy).
    .on("#divtabSUMMARY", {
      element() {
        if (summaryBuf == null) summaryBuf = "";
      },
      text(t) {
        if (summaryBuf != null) summaryBuf += t.text;
      },
    })
    .transform(html);

  const find = (label: RegExp) => details.find((d) => label.test(d.label));
  const mainTitle = find(/^main title$/i)?.values[0]?.text ?? null;
  const title = mainTitle ? mainTitle.split(" / ")[0].trim() : null;
  const seriesTitle = find(/^series/i)?.values[0]?.text ?? null;
  const isbnRow = find(/^isbn$/i)?.values[0]?.text?.match(/\d{9,13}[Xx]?/)?.[0] ?? null;
  const brnRow = find(/^brn$/i)?.values[0]?.text ?? null;
  const bookmarkUrl = find(/^bookmark/i)?.values[0]?.url ?? null;
  const brn = brnRow ?? bookmarkUrl?.match(/BRN=(\d+)/)?.[1] ?? null;

  let summary = summaryBuf != null ? collapse(summaryBuf) : null;
  if (summary && /^(loading\.?\.?\.?)?$/i.test(summary)) summary = null;

  return {
    bibIrn,
    brn,
    title,
    seriesTitle,
    isbn: isbnRow ?? isbnFromCover,
    coverUrls: covers,
    summary,
    details,
    bookmarkUrl: absolute(origin, bookmarkUrl ?? (brn ? `/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=${brn}` : null)),
    related: {
      similar: absolute(origin, related.similar ?? null),
      author: absolute(origin, related.author ?? null),
      series: absolute(origin, related.series ?? null),
      versions: absolute(origin, related.version ?? null),
    },
  };
}

// ---------------------------------------------------------------------------
// Related-titles fragment → brief cards
// ---------------------------------------------------------------------------

interface CardDraft {
  bibUrl: string | null;
  titleBuf: string;
  coverUrls: string[];
  isbn: string | null;
  lineBuf: string | null;
  detailLines: string[];
}

function newCard(): CardDraft {
  return { bibUrl: null, titleBuf: "", coverUrls: [], isbn: null, lineBuf: null, detailLines: [] };
}

function finalizeCard(d: CardDraft, origin: string): RelatedItem | null {
  const title = collapse(d.titleBuf) || null;
  if (!title || !d.bibUrl) return null;
  let author: string | null = null;
  let year: string | null = null;
  for (const raw of d.detailLines) {
    const line = collapse(raw);
    if (!line) continue;
    if (year == null && /^\d{4}(\s*[-–]\s*\d{4})?$/.test(line)) year = line;
    else if (author == null) author = line;
  }
  const brn = d.bibUrl.match(/BRN=(\d+)/)?.[1] ?? d.bibUrl.match(/IRN[=(](\d+)/)?.[1] ?? null;
  return { title, bibUrl: absolute(origin, d.bibUrl), brn, author, year, isbn: d.isbn, coverUrls: d.coverUrls };
}

export function parseBriefCards(html: string, origin: string): RelatedItem[] {
  const items: RelatedItem[] = [];
  let current: CardDraft | null = null;
  const flush = () => {
    if (current) {
      const it = finalizeCard(current, origin);
      if (it) items.push(it);
    }
  };

  new HTMLRewriter()
    // Each card opens with its cover link; use it as the card boundary.
    .on("a.thumblink", {
      element(el) {
        flush();
        current = newCard();
        current.bibUrl = el.getAttribute("href");
      },
    })
    .on("span.card-title a", {
      element(el) {
        if (!current) {
          current = newCard();
          current.bibUrl = el.getAttribute("href");
        } else if (!current.bibUrl) {
          current.bibUrl = el.getAttribute("href");
        }
      },
      text(t) {
        if (current) current.titleBuf += t.text;
      },
    })
    .on("img.imgsc", {
      element(el) {
        if (!current) return;
        const { covers, isbn } = parseCovers(el.getAttribute("data-imgurls"));
        if (covers.length) current.coverUrls = covers;
        if (isbn) current.isbn = isbn;
      },
    })
    .on(".card-text.recdetails span.d-block", {
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
    .transform(html);

  flush();
  return items;
}
