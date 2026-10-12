// Zero-dependency Spydus OPAC client (Bun fetch). This is the data source the
// parsing engine runs on, and the basis for the Hono routes later.

export const ORIGIN = "https://canning.spydus.com";

const UA =
  "Mozilla/5.0 (Linux; Android 14; Pixel 10 Pro) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0.0.0 Mobile Safari/537.36";

function baseHeaders(cookie?: string): Record<string, string> {
  const h: Record<string, string> = { "User-Agent": UA, Accept: "text/html,application/json" };
  if (cookie) h["Cookie"] = cookie;
  return h;
}

/** Pull just the PQID session cookie out of a Set-Cookie header. */
export function pqidFrom(setCookie: string | null): string | null {
  if (!setCookie) return null;
  const m = setCookie.match(/PQID=\d+/);
  return m ? m[0] : null;
}

/**
 * The native app's first call: GET /api/mobile/1.0/discover/search returns a
 * JSON-encoded OPAC path string. We let the server decide the search params
 * rather than hard-coding ALLWRKENQ, exactly like the real client.
 */
export async function resolveSearchUrl(query: string, origin = ORIGIN): Promise<string> {
  const u = `${origin}/api/mobile/1.0/discover/search?keywords=${encodeURIComponent(query)}`;
  const res = await fetch(u, { headers: { ...baseHeaders(), "spydus-lang": "" } });
  if (!res.ok) throw new Error(`discover/search ${res.status}`);
  const path = (await res.json()) as string; // e.g. "/cgi-bin/spydus.exe/ENQ/OPAC/ALLWRKENQ?..."
  return origin + path;
}

export interface FetchedPage {
  url: string;
  html: string;
  cookie: string | null;
}

/** Resolve + fetch the results page. Returns the HTML and the PQID cookie. */
export async function fetchSearch(query: string, origin = ORIGIN): Promise<FetchedPage> {
  const url = await resolveSearchUrl(query, origin);
  const res = await fetch(url, { headers: baseHeaders() });
  if (!res.ok) throw new Error(`results ${res.status}`);
  const cookie = pqidFrom(res.headers.get("set-cookie"));
  const html = await res.text();
  return { url, html, cookie };
}

/** Fetch an availability (RECFMT) or any OPAC fragment, carrying the PQID cookie. */
export async function fetchFragment(absUrl: string, cookie?: string | null): Promise<string> {
  const res = await fetch(absUrl, { headers: baseHeaders(cookie ?? undefined) });
  if (!res.ok) throw new Error(`fragment ${res.status}`);
  return res.text();
}
