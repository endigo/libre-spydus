// Spydus OPAC "RECFMT" record fragment → JSON.
//
// This is the per-record-per-format fragment the results WebView lazy-loads
// (/ENQ/OPAC/BIBENQ/{setId}?...&RECFMT=BK). It carries the live availability
// counts, publisher line, and the links to the branch-level holdings modal.

import { collapse, toInt, absolute } from "./decode";
import type { Availability } from "../types";

export interface ParseAvailabilityOptions {
  origin?: string;
}

export function parseAvailability(html: string, opts: ParseAvailabilityOptions = {}): Availability {
  const origin = opts.origin ?? "https://canning.spydus.com";

  const counts: Record<string, number> = {};
  let pendingLabel: string | null = null;
  let labelBuf = "";
  let valBuf = "";

  let titleBuf = "";
  let titleDone = false;
  const spanCandidates: string[] = [];
  let spanBuf: string | null = null;

  let holdingsUrl: string | null = null;
  let recordUrl: string | null = null;
  let bibRecordIrn: string | null = null;

  const rw = new HTMLRewriter()
    // availability label → value pairs (Reserved: 1, Copies: 3, ...)
    .on("span.availability-text", {
      element(el) {
        labelBuf = "";
        el.onEndTag(() => {
          pendingLabel = collapse(labelBuf).replace(/:$/, "").toLowerCase() || null;
        });
      },
      text(t) {
        labelBuf += t.text;
      },
    })
    .on("span.availability-val", {
      element(el) {
        valBuf = "";
        el.onEndTag(() => {
          const n = toInt(valBuf);
          if (pendingLabel && n != null) counts[pendingLabel] = n;
          pendingLabel = null;
        });
      },
      text(t) {
        valBuf += t.text;
      },
    })
    // record title — first title cell of the stacked table
    .on('td[data-caption="Title"] a', {
      text(t) {
        if (!titleDone) titleBuf += t.text;
      },
      element(el) {
        el.onEndTag(() => {
          if (collapse(titleBuf)) titleDone = true;
        });
      },
    })
    // imprint candidates — bare spans inside a column; pick the publisher line after
    .on(".recfmt-container .col > span", {
      element(el) {
        spanBuf = "";
        el.onEndTag(() => {
          const s = collapse(spanBuf ?? "");
          if (s) spanCandidates.push(s);
          spanBuf = null;
        });
      },
      text(t) {
        if (spanBuf != null) spanBuf += t.text;
      },
    })
    // branch-level holdings modal (XHLD) and full record (XFULL)
    .on('a[href*="/XHLD/OPAC/"]', {
      element(el) {
        const href = el.getAttribute("href");
        if (href && !holdingsUrl) {
          holdingsUrl = href;
          const m = href.match(/\/XHLD\/OPAC\/\w+\/\d+\/(\d+)/);
          if (m) bibRecordIrn = m[1];
        }
      },
    })
    .on('a[href*="/XFULL/OPAC/"]', {
      element(el) {
        const href = el.getAttribute("href");
        if (href && !recordUrl) {
          recordUrl = href;
          if (!bibRecordIrn) {
            const m = href.match(/\/XFULL\/OPAC\/\w+\/\d+\/(\d+)/);
            if (m) bibRecordIrn = m[1];
          }
        }
      },
    });

  rw.transform(html);

  // The publisher line is the span that looks like an imprint: a " : " separator
  // (place : publisher) and/or a 4-digit year. Pick the first such candidate.
  const publisher =
    spanCandidates.find((s) => /\s:\s/.test(s) && /\d{4}/.test(s)) ??
    spanCandidates.find((s) => /\s:\s/.test(s)) ??
    null;

  return {
    bibRecordIrn,
    title: collapse(titleBuf) || null,
    publisher,
    counts,
    reserved: counts["reserved"] ?? null,
    copies: counts["copies"] ?? null,
    holdingsUrl: absolute(origin, holdingsUrl),
    recordUrl: absolute(origin, recordUrl),
  };
}
