// Authenticated OPAC account pages → JSON.
//   parseAccountLinks  — the BRWENQ "My details" page → loans/reservations links
//   parsePatronItems   — a LOANRENQ / RSVCENQ page → list of loans/reservations
//
// Loans and reservations share one table-row layout, so a single parser handles
// both; callers interpret the `fields` map ("Due" for loans, "Status" for
// reservations) and the `actions` (Renew vs Cancel/Edit).

import { collapse, decode, toInt, absolute } from "./decode";
import type { AccountLinks, PatronItem, PatronList } from "../types";

// ---------------------------------------------------------------------------
// Account page → navigation links
// ---------------------------------------------------------------------------

export function parseAccountLinks(html: string, origin: string): AccountLinks {
  let loansUrl: string | null = null;
  let reservationsReadyUrl: string | null = null;
  let reservationsPendingUrl: string | null = null;
  let name: string | null = null;
  let nameBuf: string | null = null;

  new HTMLRewriter()
    .on("a[href]", {
      element(el) {
        const href = decode(el.getAttribute("href") ?? "");
        if (!href.includes("/OPAC/")) return;
        const fmt = href.match(/[?&]FMT=([A-Z]+)/)?.[1];
        if (href.includes("/LOANRENQ/") && fmt === "CL" && !loansUrl) {
          loansUrl = href;
        } else if (href.includes("/RSVCENQ/") && fmt === "AR" && !reservationsReadyUrl) {
          reservationsReadyUrl = href;
        } else if (href.includes("/RSVCENQ/") && fmt === "WR" && !reservationsPendingUrl) {
          reservationsPendingUrl = href;
        }
      },
    })
    // Logged-in borrower name — the navbar account dropdown toggle.
    // #navbarLoginMenuLinkName: the signed-in navbar link on current Spydus templates ("First Last").
    .on(".navbar .dropdown-toggle-account, .account-name, #accountDropdown, #navbarLoginMenuLinkName", {
      element() {
        if (name == null) nameBuf = "";
      },
      text(t) {
        if (nameBuf != null) nameBuf += t.text;
      },
    })
    .transform(html);

  if (nameBuf != null) {
    const n = collapse(nameBuf);
    if (n) name = n;
  }

  return {
    name,
    loansUrl: absolute(origin, loansUrl),
    reservationsReadyUrl: absolute(origin, reservationsReadyUrl),
    reservationsPendingUrl: absolute(origin, reservationsPendingUrl),
  };
}

// ---------------------------------------------------------------------------
// Loan / reservation list → items
// ---------------------------------------------------------------------------

interface Draft {
  irn: string | null;
  titleBuf: string;
  bibUrl: string | null;
  detailLines: string[];
  lineBuf: string | null;
  coverUrls: string[];
  isbn: string | null;
  fields: Record<string, string>;
  capCaption: string | null;
  capBuf: string;
  actions: { label: string; href: string }[];
  actBuf: string | null;
  actHref: string | null;
}

function newDraft(): Draft {
  return {
    irn: null,
    titleBuf: "",
    bibUrl: null,
    detailLines: [],
    lineBuf: null,
    coverUrls: [],
    isbn: null,
    fields: {},
    capCaption: null,
    capBuf: "",
    actions: [],
    actBuf: null,
    actHref: null,
  };
}

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

function finalize(d: Draft, origin: string): PatronItem {
  let author: string | null = null;
  let year: string | null = null;
  for (const raw of d.detailLines) {
    const line = collapse(raw);
    if (!line) continue;
    if (year == null && /^\d{4}(\s*[-–]\s*\d{4})?$/.test(line)) {
      year = line;
    } else if (author == null) {
      author = line;
    }
  }
  // The bib-record IRN lives inside the title link, as QRY=IRN(nnn) or IRN=nnn.
  const bibIrn = d.bibUrl?.match(/IRN[=(](\d+)/)?.[1] ?? null;
  return {
    irn: d.irn,
    bibIrn,
    title: collapse(d.titleBuf) || null,
    bibUrl: absolute(origin, d.bibUrl),
    author,
    year,
    coverUrls: d.coverUrls,
    isbn: d.isbn,
    fields: d.fields,
    actions: d.actions,
  };
}

export function parsePatronItems(html: string, origin: string): PatronList {
  const items: PatronItem[] = [];
  let current: Draft | null = null;
  let headerBuf = "";

  new HTMLRewriter()
    .on(".result-header-brief", {
      text(t) {
        headerBuf += t.text;
      },
    })
    // Each data row is a <tr> carrying an SVL checkbox. Header rows (no SVL) are
    // opened as drafts too but dropped at finalize because they have no irn.
    .on("tr", {
      element(el) {
        const d = newDraft();
        current = d;
        el.onEndTag(() => {
          if (d.irn && collapse(d.titleBuf)) items.push(finalize(d, origin));
          if (current === d) current = null;
        });
      },
    })
    .on('tr input[name="SVL"]', {
      element(el) {
        if (current && !current.irn) current.irn = el.getAttribute("value");
      },
    })
    .on("tr h3.card-title a", {
      element(el) {
        if (current && !current.bibUrl) current.bibUrl = el.getAttribute("href");
      },
      text(t) {
        if (current) current.titleBuf += t.text;
      },
    })
    .on("tr div.card-text > span.d-block", {
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
    .on("tr img.imgsc", {
      element(el) {
        if (!current) return;
        const { covers, isbn } = parseCovers(el.getAttribute("data-imgurls"));
        if (covers.length) current.coverUrls = covers;
        if (isbn) current.isbn = isbn;
      },
    })
    // Columnar cells: <td data-caption="Due|Status">…</td>
    .on("tr td[data-caption]", {
      element(el) {
        if (!current) return;
        current.capCaption = el.getAttribute("data-caption");
        current.capBuf = "";
        el.onEndTag(() => {
          if (current && current.capCaption) {
            current.fields[current.capCaption] = collapse(current.capBuf);
            current.capCaption = null;
          }
        });
      },
      text(t) {
        if (current && current.capCaption != null) current.capBuf += t.text;
      },
    })
    // Row action links (Renew / Cancel / Edit).
    .on("tr a.dropdown-item", {
      element(el) {
        if (!current) return;
        current.actHref = decode(el.getAttribute("href") ?? "");
        current.actBuf = "";
        el.onEndTag(() => {
          if (current && current.actHref) {
            const label = collapse(current.actBuf ?? "");
            const href = absolute(origin, current.actHref);
            if (href && label && !current.actions.some((a) => a.href === href)) {
              current.actions.push({ label, href });
            }
            current.actHref = null;
            current.actBuf = null;
          }
        });
      },
      text(t) {
        if (current && current.actBuf != null) current.actBuf += t.text;
      },
    })
    .transform(html);

  const m = collapse(headerBuf).match(/of\s+([\d,]+)/i);
  return { total: m ? toInt(m[1]) : items.length, items };
}
