// Tiny, dependency-free helpers for turning raw HTML text/attribute chunks into
// clean values. HTMLRewriter already decodes most character references in text
// nodes, but attribute values (hrefs with `&amp;`) and stray entities still need
// a pass, so `decode` is written to be idempotent.

const NAMED: Record<string, string> = {
  amp: "&",
  lt: "<",
  gt: ">",
  quot: '"',
  apos: "'",
  nbsp: " ",
  copy: "©",
  reg: "®",
  hellip: "…",
  mdash: "—",
  ndash: "–",
  rsquo: "’",
  lsquo: "‘",
  ldquo: "“",
  rdquo: "”",
};

/** Decode HTML entities (named + numeric, decimal and hex). Idempotent. */
export function decode(input: string): string {
  if (!input || input.indexOf("&") === -1) return input;
  return input.replace(/&(#x?[0-9a-f]+|[a-z][a-z0-9]*);/gi, (whole, body: string) => {
    if (body[0] === "#") {
      const code =
        body[1] === "x" || body[1] === "X"
          ? parseInt(body.slice(2), 16)
          : parseInt(body.slice(1), 10);
      return Number.isFinite(code) ? safeFromCodePoint(code) : whole;
    }
    const hit = NAMED[body.toLowerCase()];
    return hit ?? whole;
  });
}

function safeFromCodePoint(code: number): string {
  try {
    return String.fromCodePoint(code);
  } catch {
    return "";
  }
}

/** Collapse runs of whitespace to single spaces and trim. */
export function collapse(input: string): string {
  return decode(input).replace(/\s+/g, " ").trim();
}

/** First integer found in a string, or null. */
export function toInt(input: string | null | undefined): number | null {
  if (!input) return null;
  const m = input.replace(/,/g, "").match(/-?\d+/);
  return m ? parseInt(m[0], 10) : null;
}

/** Normalise a relative Spydus path to an absolute URL on the given origin. */
export function absolute(origin: string, href: string | null | undefined): string | null {
  if (!href) return null;
  const clean = decode(href).trim();
  if (!clean) return null;
  if (/^https?:\/\//i.test(clean)) return clean;
  return origin.replace(/\/$/, "") + (clean.startsWith("/") ? clean : "/" + clean);
}
