// Spydus → JSON middleware (Bun + Hono).
//
//   GET /health                 liveness + upstream origin
//   GET /search?q=...           OPAC keyword search → SearchResults JSON (list)
//   GET /availability?url=...   one RECFMT fragment → Availability JSON (counts)
//
// The search list is cached ~10 min; availability ~60 s. The list omits live
// counts on purpose — the client fetches /availability per card (lazy), exactly
// like the real app's WebView, so first paint is a single upstream fetch.

import { Hono } from "hono";
import { cors } from "hono/cors";
import { fetchSearch, fetchFragment, ORIGIN } from "./fetch/spydus";
import {
  login,
  resolveAccount,
  fetchAuthed,
  reserveOptions,
  placeReservation,
  cancelReservation,
  editReservationOptions,
  editReservation,
} from "./fetch/auth";
import { parseSearchResults } from "./parse/search-results";
import { parseAvailability } from "./parse/availability";
import { parsePatronItems } from "./parse/account";
import { parseRecordDetail, parseBriefCards, bibLinkFromFragment } from "./parse/record";
import type { AccountLinks } from "./types";

/** Fetch + parse a results-set page (either the initial search or a SET page). */
async function parsePage(url: string, query?: string) {
  const html = await fetchFragment(url, null);
  return parseSearchResults(html, query ? { query } : {});
}

const app = new Hono();
app.use("*", cors());

// --- tiny in-memory TTL cache (also de-dupes concurrent identical requests) ---
const cache = new Map<string, { exp: number; val: Promise<unknown> }>();
function cached<T>(key: string, ttlMs: number, fn: () => Promise<T>): Promise<T> {
  const now = Date.now();
  const hit = cache.get(key);
  if (hit && hit.exp > now) return hit.val as Promise<T>;
  const val = fn().catch((e) => {
    cache.delete(key); // don't cache failures
    throw e;
  });
  cache.set(key, { exp: now + ttlMs, val });
  return val;
}

app.get("/health", (c) => c.json({ ok: true, origin: ORIGIN }));

app.get("/search", async (c) => {
  const q = c.req.query("q")?.trim();
  if (!q) return c.json({ error: "missing query param 'q'" }, 400);
  try {
    const data = await cached(`s:${q.toLowerCase()}`, 10 * 60_000, async () => {
      const page = await fetchSearch(q);
      return parseSearchResults(page.html, { query: q });
    });
    return c.json(data);
  } catch (e) {
    return c.json({ error: "upstream search failed", detail: String(e) }, 502);
  }
});

app.get("/page", async (c) => {
  const url = c.req.query("url");
  if (!url) return c.json({ error: "missing query param 'url'" }, 400);
  if (!url.startsWith(ORIGIN + "/")) return c.json({ error: "forbidden origin" }, 403);
  try {
    const data = await cached(`p:${url}`, 10 * 60_000, () => parsePage(url));
    return c.json(data);
  } catch (e) {
    return c.json({ error: "upstream page failed", detail: String(e) }, 502);
  }
});

app.get("/availability", async (c) => {
  const url = c.req.query("url");
  if (!url) return c.json({ error: "missing query param 'url'" }, 400);
  // SSRF guard: only fetch fragments from the configured library origin.
  if (!url.startsWith(ORIGIN + "/")) return c.json({ error: "forbidden origin" }, 403);
  try {
    const data = await cached(`a:${url}`, 60_000, async () => {
      const html = await fetchFragment(url, null);
      return parseAvailability(html);
    });
    return c.json(data);
  } catch (e) {
    return c.json({ error: "upstream availability failed", detail: String(e) }, 502);
  }
});

// Rich bib record: resolve the format fragment → concrete bib page → details.
// Pass a search record's format `availabilityUrl` (BK preferred) as `url`.
app.get("/record", async (c) => {
  const url = c.req.query("url");
  if (!url) return c.json({ error: "missing query param 'url'" }, 400);
  if (!url.startsWith(ORIGIN + "/")) return c.json({ error: "forbidden origin" }, 403);
  try {
    const data = await cached(`rec:${url}`, 10 * 60_000, async () => {
      // A RECFMT fragment must be resolved to its bib page first; a direct
      // BIBENQ bib URL (e.g. a related-title link) is parsed as-is.
      if (/[?&]FMT=RECFMT/.test(url)) {
        const frag = await fetchFragment(url, null);
        const bib = bibLinkFromFragment(frag, ORIGIN);
        if (!bib) throw new Error("no bib record link in format fragment");
        const html = await fetchFragment(bib.bibUrl, null);
        return parseRecordDetail(html, ORIGIN, bib.bibIrn);
      }
      const html = await fetchFragment(url, null);
      const bibIrn = url.match(/IRN[=(](\d+)/)?.[1] ?? null;
      return parseRecordDetail(html, ORIGIN, bibIrn);
    });
    return c.json(data);
  } catch (e) {
    return c.json({ error: "upstream record failed", detail: String(e) }, 502);
  }
});

// A related-titles panel (similar / author / series / versions) → brief cards.
// Pass one of RecordDetail.related.* as `url`.
app.get("/related", async (c) => {
  const url = c.req.query("url");
  if (!url) return c.json({ error: "missing query param 'url'" }, 400);
  if (!url.startsWith(ORIGIN + "/")) return c.json({ error: "forbidden origin" }, 403);
  try {
    const data = await cached(`rel:${url}`, 10 * 60_000, async () => {
      const html = await fetchFragment(url, null);
      return { items: parseBriefCards(html, ORIGIN) };
    });
    return c.json(data);
  } catch (e) {
    return c.json({ error: "upstream related failed", detail: String(e) }, 502);
  }
});

// --- Authenticated account (My loans / My reservations) -------------------

/** Resolve the account links for a session, cached briefly (loans + reservations
 *  views arrive back-to-back and each would otherwise re-walk the dashboard). */
function accountLinks(session: string): Promise<AccountLinks> {
  return cached(`acct:${session}`, 60_000, () => resolveAccount(session));
}

function sessionOf(c: { req: { header: (n: string) => string | undefined } }): string | undefined {
  return c.req.header("X-Spydus-Session")?.trim() || undefined;
}

app.post("/login", async (c) => {
  let creds: { username?: string; password?: string };
  try {
    creds = await c.req.json();
  } catch {
    return c.json({ error: "expected JSON body { username, password }" }, 400);
  }
  const username = creds.username?.trim();
  const password = creds.password;
  if (!username || !password) return c.json({ error: "username and password required" }, 400);
  try {
    const { session } = await login(username, password);
    // Confirm the session can actually read the account before returning it.
    await accountLinks(session);
    return c.json({ ok: true, session });
  } catch (e) {
    const msg = String(e);
    // Credentials rejected by the OPAC (bad id/PIN, or this id isn't accepted)
    // → 401 with a user-facing reason. Anything else (upstream/network) → 502.
    if (msg.includes("credentials rejected") || msg.includes("not logged in")) {
      return c.json(
        { ok: false, error: "credentials_rejected", reason: "Your login ID or PIN was not accepted." },
        401,
      );
    }
    return c.json(
      { ok: false, error: "upstream_error", reason: "Couldn't reach the library service.", detail: msg },
      502,
    );
  }
});

// Reservation request form (pickup location + expiry choices). Read-only — no
// hold is placed. `url` is a search record's format `reserveUrl`.
app.get("/reserve/options", async (c) => {
  const session = sessionOf(c);
  if (!session) return c.json({ error: "missing X-Spydus-Session" }, 401);
  const url = c.req.query("url");
  if (!url) return c.json({ error: "missing query param 'url'" }, 400);
  if (!url.startsWith(ORIGIN + "/")) return c.json({ error: "forbidden origin" }, 403);
  try {
    return c.json(await reserveOptions(url, session));
  } catch (e) {
    return c.json({ error: "could not load reservation form", detail: String(e) }, 502);
  }
});

// Place a reservation. MUTATING — only on explicit user confirmation.
app.post("/reserve", async (c) => {
  const session = sessionOf(c);
  if (!session) return c.json({ ok: false, message: "Please sign in." }, 401);
  let payload: { url?: string; xdays?: string; pickup?: string };
  try {
    payload = await c.req.json();
  } catch {
    return c.json({ ok: false, message: "expected JSON body { url }" }, 400);
  }
  const url = payload.url;
  if (!url) return c.json({ ok: false, message: "missing 'url'" }, 400);
  if (!url.startsWith(ORIGIN + "/")) return c.json({ ok: false, message: "forbidden origin" }, 403);
  try {
    const result = await placeReservation(url, session, { xdays: payload.xdays, pickup: payload.pickup });
    // A freshly placed hold invalidates the cached reservations for this session.
    cache.delete(`acct:${session}`);
    return c.json(result, result.ok ? 200 : 409);
  } catch (e) {
    return c.json({ ok: false, message: "Reservation failed.", detail: String(e) }, 502);
  }
});

// Cancel a reservation. MUTATING — only on explicit user confirmation. `url` is
// a reservation row's "Cancel reservation" action href (…/C/5/RSVCENQ?SVL=…).
app.post("/reserve/cancel", async (c) => {
  const session = sessionOf(c);
  if (!session) return c.json({ ok: false, message: "Please sign in." }, 401);
  let payload: { url?: string };
  try {
    payload = await c.req.json();
  } catch {
    return c.json({ ok: false, message: "expected JSON body { url }" }, 400);
  }
  const url = payload.url;
  if (!url) return c.json({ ok: false, message: "missing 'url'" }, 400);
  if (!url.startsWith(ORIGIN + "/")) return c.json({ ok: false, message: "forbidden origin" }, 403);
  try {
    const result = await cancelReservation(url, session);
    cache.delete(`acct:${session}`);
    return c.json(result, result.ok ? 200 : 409);
  } catch (e) {
    return c.json({ ok: false, message: "Cancel failed.", detail: String(e) }, 502);
  }
});

// Edit-reservation form (pickup locations + current dates). Read-only — the GET
// renders the form and makes no change. `url` is the "Edit reservation" href.
app.get("/reserve/edit", async (c) => {
  const session = sessionOf(c);
  if (!session) return c.json({ error: "missing X-Spydus-Session" }, 401);
  const url = c.req.query("url");
  if (!url) return c.json({ error: "missing query param 'url'" }, 400);
  if (!url.startsWith(ORIGIN + "/")) return c.json({ error: "forbidden origin" }, 403);
  try {
    return c.json(await editReservationOptions(url, session));
  } catch (e) {
    return c.json({ error: "could not load edit form", detail: String(e) }, 502);
  }
});

// Edit a reservation. MUTATING — only on explicit user confirmation.
app.post("/reserve/edit", async (c) => {
  const session = sessionOf(c);
  if (!session) return c.json({ ok: false, message: "Please sign in." }, 401);
  let payload: {
    url?: string;
    pickup?: string;
    delayUntil?: string;
    notRequiredAfter?: string;
    clearDelay?: boolean;
    clearExpiry?: boolean;
  };
  try {
    payload = await c.req.json();
  } catch {
    return c.json({ ok: false, message: "expected JSON body { url }" }, 400);
  }
  const url = payload.url;
  if (!url) return c.json({ ok: false, message: "missing 'url'" }, 400);
  if (!url.startsWith(ORIGIN + "/")) return c.json({ ok: false, message: "forbidden origin" }, 403);
  try {
    const result = await editReservation(url, session, {
      pickup: payload.pickup,
      delayUntil: payload.delayUntil,
      notRequiredAfter: payload.notRequiredAfter,
      clearDelay: payload.clearDelay,
      clearExpiry: payload.clearExpiry,
    });
    cache.delete(`acct:${session}`);
    return c.json(result, result.ok ? 200 : 409);
  } catch (e) {
    return c.json({ ok: false, message: "Edit failed.", detail: String(e) }, 502);
  }
});

app.get("/account/loans", async (c) => {
  const session = sessionOf(c);
  if (!session) return c.json({ error: "missing X-Spydus-Session" }, 401);
  try {
    const links = await accountLinks(session);
    if (!links.loansUrl) return c.json({ total: 0, items: [] });
    const html = await fetchAuthed(links.loansUrl, session);
    return c.json(parsePatronItems(html, ORIGIN));
  } catch (e) {
    return c.json({ error: "could not load loans", detail: String(e) }, 502);
  }
});

app.get("/account/reservations", async (c) => {
  const session = sessionOf(c);
  if (!session) return c.json({ error: "missing X-Spydus-Session" }, 401);
  try {
    const links = await accountLinks(session);
    const [ready, pending] = await Promise.all([
      links.reservationsReadyUrl
        ? fetchAuthed(links.reservationsReadyUrl, session).then((h) => parsePatronItems(h, ORIGIN))
        : Promise.resolve({ total: 0, items: [] }),
      links.reservationsPendingUrl
        ? fetchAuthed(links.reservationsPendingUrl, session).then((h) => parsePatronItems(h, ORIGIN))
        : Promise.resolve({ total: 0, items: [] }),
    ]);
    return c.json({ ready, pending });
  } catch (e) {
    return c.json({ error: "could not load reservations", detail: String(e) }, 502);
  }
});

const port = Number(process.env.PORT ?? 8787);
console.log(`spydus middleware on http://0.0.0.0:${port}  → ${ORIGIN}`);

export default {
  port,
  // Upstream renders can take 5–9 s; keep sockets alive well past that.
  idleTimeout: 120,
  fetch: app.fetch,
};
