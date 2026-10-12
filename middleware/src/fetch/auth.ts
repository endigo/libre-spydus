// Authenticated OPAC session: login (RSA-encrypted password, as the web client
// does), authed fetch, and account-page resolution.
//
// The SPYDUS_BRWSESSIONID cookie is the session credential. The middleware is
// stateless: the app holds the session and sends it back on each account call.

import { publicEncrypt, constants } from "node:crypto";
import { ORIGIN } from "./spydus";
import { parseAccountLinks } from "../parse/account";
import {
  parseReserveForm,
  toReserveOptions,
  parseReserveResult,
  parseEditForm,
  toEditOptions,
  parseManageResult,
} from "../parse/reserve";
import type { AccountLinks, ReserveOptions, ReserveResult, EditOptions } from "../types";

const UA =
  "Mozilla/5.0 (Linux; Android 14; Pixel 10 Pro) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0.0.0 Mobile Safari/537.36";

const SESSION_COOKIE = "SPYDUS_BRWSESSIONID_443";
// UI cookies the native app assigns (method 0x060007d6); authed reads need them.
const UI_COOKIES = "SPYDUS_DISPLAYCONTEXT_443=MOB; consentSettings_443=ALL; SPYDUS_MYACCOUNT_443=";

const LOGIN_POST_PATH = "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/LB/2";
const LOGIN_RDT = "/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/LB/1?ISGLB=0";
const ENCRYPT_JS = "/docs/OPAC/js/encrypt.js";

/** Fetch the OPAC RSA public key (same key JSEncrypt uses in the browser). */
export async function getPublicKey(origin = ORIGIN): Promise<string> {
  const res = await fetch(origin + ENCRYPT_JS, { headers: { "User-Agent": UA } });
  if (!res.ok) throw new Error(`encrypt.js ${res.status}`);
  const js = await res.text();
  const m = js.match(/-----BEGIN PUBLIC KEY-----([\s\S]*?)-----END PUBLIC KEY-----/);
  if (!m) throw new Error("public key not found in encrypt.js");
  const body = m[1].replace(/[\\\s]/g, ""); // strip JS line-continuations + whitespace
  const lines = body.match(/.{1,64}/g)!.join("\n");
  return `-----BEGIN PUBLIC KEY-----\n${lines}\n-----END PUBLIC KEY-----`;
}

/** RSA PKCS#1 v1.5 encrypt → base64, matching JSEncrypt.encrypt(). */
export function rsaEncrypt(pem: string, value: string): string {
  return publicEncrypt(
    { key: pem, padding: constants.RSA_PKCS1_PADDING },
    Buffer.from(value, "utf8"),
  ).toString("base64");
}

function getSetCookies(res: Response): string[] {
  return typeof (res.headers as { getSetCookie?: () => string[] }).getSetCookie === "function"
    ? (res.headers as { getSetCookie: () => string[] }).getSetCookie()
    : [res.headers.get("set-cookie") ?? ""].filter(Boolean);
}

// Match the browse-session cookie regardless of the port suffix Spydus appends
// (`SPYDUS_BRWSESSIONID` vs `SPYDUS_BRWSESSIONID_443`). A cleared cookie has an
// empty value + a past `expires`, so require a non-empty token value.
const SESSION_RE = /SPYDUS_BRWSESSIONID(?:_\d+)?=([^;\s]+)/;

function sessionFromCookies(jar: string[]): string | null {
  for (const c of jar) {
    const m = c.match(SESSION_RE);
    if (m && m[1]) return m[1];
  }
  return null;
}

/** Names only, for safe diagnostics (never the values). */
function cookieNames(jar: string[]): string[] {
  return jar.map((c) => c.split("=", 1)[0]?.trim()).filter(Boolean) as string[];
}

export interface LoginResult {
  session: string;
}

// Toggle with LOGIN_DEBUG=1 — logs upstream status / cookie NAMES / redirect
// target only. Never logs username, password, or the session token value.
const LOGIN_DEBUG = process.env.LOGIN_DEBUG === "1";

/**
 * Perform the OPAC login. Returns the session on success, or throws with a
 * credentials error. The password is RSA-encrypted before leaving this process
 * and is never logged or stored.
 *
 * Success path: LB/2 either sets the browse-session cookie directly or 302s to
 * RDT with it; failure re-renders the login form (200, no session cookie). We
 * scan the POST response and follow one redirect hop, so a valid login is never
 * lost to a cookie-name/redirect technicality.
 */
export async function login(username: string, password: string, origin = ORIGIN): Promise<LoginResult> {
  const pem = await getPublicKey(origin);
  const body = new URLSearchParams({
    BRWLID: username,
    BRWLPWD: rsaEncrypt(pem, password),
    RDT: LOGIN_RDT,
  });
  const res = await fetch(origin + LOGIN_POST_PATH, {
    method: "POST",
    headers: { "User-Agent": UA, Cookie: UI_COOKIES },
    body,
    redirect: "manual",
  });

  let jar = getSetCookies(res);
  let session = sessionFromCookies(jar);
  const location = res.headers.get("location");

  // If the cookie wasn't on the POST response but the server is redirecting us
  // on (success → RDT), follow one hop carrying the cookies set so far.
  if (!session && res.status >= 300 && res.status < 400 && location) {
    const next = location.startsWith("http") ? location : origin + location;
    const carried = [UI_COOKIES, ...jar.map((c) => c.split(";", 1)[0])].join("; ");
    const hop = await fetch(next, { method: "GET", headers: { "User-Agent": UA, Cookie: carried }, redirect: "manual" });
    jar = [...jar, ...getSetCookies(hop)];
    session = sessionFromCookies(jar);
  }

  if (LOGIN_DEBUG) {
    const bodyLen = res.headers.get("content-length") ?? "(chunked)";
    console.log(
      `[login] upstream status=${res.status} location=${location ?? "-"} ` +
        `setCookieNames=[${cookieNames(jar).join(",")}] sessionFound=${session ? "yes" : "NO"} clen=${bodyLen}`,
    );
  }

  if (!session) {
    // No session cookie issued → bad BRWLID/PIN, or this login id isn't accepted
    // (e.g. a web username where the barcode is required). Don't leak specifics.
    throw new Error("login failed: credentials rejected (no session issued)");
  }
  return { session };
}

function cookieHeader(session: string): string {
  return `${SESSION_COOKIE}=${session}; ${UI_COOKIES}`;
}

/** Authenticated GET of an OPAC read page (never an action/mutation link). */
export async function fetchAuthed(url: string, session: string): Promise<string> {
  const res = await fetch(url, { headers: { "User-Agent": UA, Cookie: cookieHeader(session) } });
  if (!res.ok) throw new Error(`authed GET ${res.status}`);
  return res.text();
}

// --- Reservations (place a hold) ------------------------------------------
//
// Flow: GET the reserve URL (follows the CCOPT V→R/9 wizard to the "Place
// reservation" form) → serialize it (csrf, PICKUP, ITM, XDAYS) → POST to commit,
// following the wizard until it leaves the form. The GET is side-effect-free;
// only the POST places the hold. Gated by RESERVE_DEBUG for safe logging.

const RESERVE_DEBUG = process.env.RESERVE_DEBUG === "1";

/** Read the reservation request form (no hold is placed). */
export async function reserveOptions(reserveUrl: string, session: string): Promise<ReserveOptions> {
  const html = await fetchAuthed(reserveUrl, session);
  // A logged-out session lands on the login form, not the reserve form.
  if (/id="frmLogin"/.test(html)) {
    return { canReserve: false, pickupLocation: null, availableSummary: null, expiryChoices: [], message: "Please sign in again." };
  }
  return toReserveOptions(parseReserveForm(html, ORIGIN));
}

export interface ReserveOverrides {
  xdays?: string;
  pickup?: string;
}

/** Place a reservation. Mutating — only called when the user confirms. */
export async function placeReservation(
  reserveUrl: string,
  session: string,
  overrides: ReserveOverrides = {},
): Promise<ReserveResult> {
  const cookie = cookieHeader(session);
  let formHtml = await fetchAuthed(reserveUrl, session);
  let prevAction = "";

  // Submit the form, then auto-continue through up to 2 further wizard forms
  // (e.g. a confirmation step). Stop when the same step repeats (a validation
  // error) or the page is no longer a reserve form.
  for (let step = 0; step < 3; step++) {
    const form = parseReserveForm(formHtml, ORIGIN);
    if (!form) break;
    if (form.action === prevAction) break; // same step again → not progressing
    prevAction = form.action;

    const body = { ...form.body };
    if (step === 0) {
      if (overrides.xdays != null) body["XDAYS"] = overrides.xdays;
      if (overrides.pickup != null) body["PICKUP"] = overrides.pickup;
    }

    let res = await fetch(form.action, {
      method: "POST",
      headers: { "User-Agent": UA, Cookie: cookie, "Content-Type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams(body),
      redirect: "manual",
    });
    // Follow redirects (success usually 302s to the reservations list).
    for (let hop = 0; hop < 4 && res.status >= 300 && res.status < 400; hop++) {
      const loc = res.headers.get("location");
      if (!loc) break;
      res = await fetch(loc.startsWith("http") ? loc : ORIGIN + loc, {
        headers: { "User-Agent": UA, Cookie: cookie },
        redirect: "manual",
      });
    }
    formHtml = await res.text();
    if (RESERVE_DEBUG) {
      console.log(`[reserve] step=${step} status=${res.status} len=${formHtml.length} stillForm=${/id="mainForm"/.test(formHtml)}`);
    }
    // No reserve form left → we've reached the result page.
    if (!/id="mainForm"/.test(formHtml)) break;
  }

  return parseReserveResult(formHtml);
}

// --- Cancel / edit an existing reservation ---------------------------------
//
// Cancel: GET …/C/5/RSVCENQ?SVL=  (renders a confirm form, side-effect-free —
//   verified live: reservation count unchanged) → POST …/C/6/RSVCENQ to commit.
// Edit:   GET …/ER/1/RSVCENQ?SVL= (renders the edit form) → POST …/ER/2/RSVCENQ.
// Both carry a mandatory per-form csrf and submitButton=Continue. The GET sets
// the SVL selection server-side; the POST commits it. Only the POST mutates.

/** POST a CCOPT form body and follow up to 4 redirects to the result page. */
async function postFormFollow(action: string, body: Record<string, string>, cookie: string): Promise<{ status: number; html: string }> {
  let res = await fetch(action, {
    method: "POST",
    headers: { "User-Agent": UA, Cookie: cookie, "Content-Type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams(body),
    redirect: "manual",
  });
  for (let hop = 0; hop < 4 && res.status >= 300 && res.status < 400; hop++) {
    const loc = res.headers.get("location");
    if (!loc) break;
    res = await fetch(loc.startsWith("http") ? loc : ORIGIN + loc, {
      headers: { "User-Agent": UA, Cookie: cookie },
      redirect: "manual",
    });
  }
  return { status: res.status, html: await res.text() };
}

/** Cancel a reservation. Mutating — only called when the user confirms. */
export async function cancelReservation(cancelUrl: string, session: string): Promise<ReserveResult> {
  const cookie = cookieHeader(session);
  const confirmHtml = await fetchAuthed(cancelUrl, session); // GET C/5 → confirm form
  if (/id="frmLogin"/.test(confirmHtml)) return { ok: false, message: "Please sign in again." };
  const form = parseReserveForm(confirmHtml, ORIGIN);
  if (!form) return { ok: false, message: "This reservation can't be cancelled right now." };

  const { status, html } = await postFormFollow(form.action, form.body, cookie); // POST C/6
  if (RESERVE_DEBUG) {
    console.log(`[cancel] status=${status} len=${html.length} stillForm=${/id="mainForm"/.test(html)}`);
  }
  return parseManageResult(html, "cancel");
}

/** Read the edit-reservation form (pickup location + delay/expiry). No change is made. */
export async function editReservationOptions(editUrl: string, session: string): Promise<EditOptions> {
  const html = await fetchAuthed(editUrl, session); // GET ER/1
  if (/id="frmLogin"/.test(html)) {
    return { canEdit: false, pickupChoices: [], pickupSelected: null, delayUntil: null, notRequiredAfter: null, message: "Please sign in again." };
  }
  return toEditOptions(parseEditForm(html, ORIGIN));
}

export interface EditOverrides {
  pickup?: string;
  delayUntil?: string;
  notRequiredAfter?: string;
  clearDelay?: boolean;
  clearExpiry?: boolean;
}

/** Edit a reservation (pickup / dates). Mutating — only called when the user confirms. */
export async function editReservation(editUrl: string, session: string, ov: EditOverrides = {}): Promise<ReserveResult> {
  const cookie = cookieHeader(session);
  const formHtml = await fetchAuthed(editUrl, session); // GET ER/1 → edit form
  if (/id="frmLogin"/.test(formHtml)) return { ok: false, message: "Please sign in again." };
  const form = parseEditForm(formHtml, ORIGIN);
  if (!form) return { ok: false, message: "This reservation can't be edited right now." };

  const body = { ...form.body };
  if (ov.pickup != null) body["PICKUP"] = ov.pickup;
  // A blank date means "no change"; the CLR flags explicitly remove an existing one.
  if (ov.delayUntil != null) body["SDTE"] = ov.delayUntil;
  if (ov.notRequiredAfter != null) body["XDTE"] = ov.notRequiredAfter;
  if (ov.clearDelay) body["CLRSDTE"] = "1";
  if (ov.clearExpiry) body["CLRXDTE"] = "1";

  const { status, html } = await postFormFollow(form.action, body, cookie); // POST ER/2
  if (RESERVE_DEBUG) {
    console.log(`[edit] status=${status} len=${html.length} stillForm=${/id="mainForm"/.test(html)}`);
  }
  return parseManageResult(html, "edit");
}

/**
 * Resolve the account entry: dashboard (meta-refresh) → BRWENQ "My details"
 * page → the loans/reservations links. Throws if the session is not logged in.
 */
export async function resolveAccount(session: string, origin = ORIGIN): Promise<AccountLinks> {
  const entry = await fetchAuthed(origin + LOGIN_RDT, session);
  const refresh = entry.match(/http-equiv="Refresh"[^>]*url=([^"'>]+)/i)?.[1];
  if (!refresh) throw new Error("not logged in (no account redirect)");
  // Canning serves the correct context under OPAC; normalise any WQUP target.
  const brwenqUrl = refresh.replace(/\/ENQ\/[A-Z]+\/BRWENQ\//, "/ENQ/OPAC/BRWENQ/");
  const accountHtml = await fetchAuthed(brwenqUrl, session);
  return parseAccountLinks(accountHtml, origin);
}
