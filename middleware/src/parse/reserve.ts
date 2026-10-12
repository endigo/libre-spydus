// The OPAC "Place reservation" form (CCOPT wizard) → a submittable field set,
// and the result page → ok/message.
//
// Reservations are mutations: the middleware never GETs or POSTs these unless a
// signed-in caller explicitly asks. The GET of the form is side-effect-free
// (verified live: it only renders the request form); the POST places the hold.

import { collapse, decode, absolute } from "./decode";
import type { ReserveOptions, ReserveResult, EditOptions, PickupChoice } from "../types";

export interface ReserveForm {
  /** Absolute POST target (/PGM/OPAC/CCOPT/{setId}/{brwId}/R/1). */
  action: string;
  /** Pre-filled fields to submit (hidden inputs, defaults, csrf, submit). */
  body: Record<string, string>;
  pickupLocation: string | null;
  availableSummary: string | null;
  expiryChoices: { days: string; label: string }[];
}

/** Serialize the `#mainForm` reservation-request form. Returns null if absent. */
export function parseReserveForm(html: string, origin: string): ReserveForm | null {
  const idx = html.indexOf('id="mainForm"');
  if (idx < 0) return null;
  const openStart = html.lastIndexOf("<form", idx);
  const openEnd = html.indexOf(">", idx);
  const closeIdx = html.indexOf("</form>", openEnd);
  const openTag = html.slice(openStart, openEnd + 1);
  const region = html.slice(openEnd + 1, closeIdx < 0 ? undefined : closeIdx);

  const action = absolute(origin, decode(openTag.match(/action="([^"]*)"/)?.[1] ?? "")) ?? "";
  const body: Record<string, string> = {};

  // Named inputs (hidden/text/date). Checkboxes/radios only when checked.
  for (const m of region.matchAll(/<input\b[^>]*>/g)) {
    const tag = m[0];
    const name = tag.match(/\bname="([^"]+)"/)?.[1];
    if (!name) continue;
    const type = (tag.match(/\btype="([^"]+)"/)?.[1] ?? "text").toLowerCase();
    if (type === "checkbox" || type === "radio") {
      if (/\bchecked\b/.test(tag)) body[name] = decode(tag.match(/\bvalue="([^"]*)"/)?.[1] ?? "on");
      continue;
    }
    body[name] = decode(tag.match(/\bvalue="([^"]*)"/)?.[1] ?? "");
  }

  // Selects → selected option, else first option.
  for (const sm of region.matchAll(/<select\b[^>]*\bname="([^"]+)"[^>]*>([\s\S]*?)<\/select>/g)) {
    const name = sm[1];
    const inner = sm[2];
    const selected =
      inner.match(/<option[^>]*\bselected\b[^>]*\bvalue="([^"]*)"/)?.[1] ??
      inner.match(/<option[^>]*\bvalue="([^"]*)"[^>]*\bselected\b/)?.[1];
    const first = inner.match(/<option[^>]*\bvalue="([^"]*)"/)?.[1] ?? "";
    body[name] = decode(selected ?? first);
  }

  // Primary submit button (name=value) — Spydus expects submitButton=Continue.
  const sb = region.match(/<button[^>]*type="submit"[^>]*\bname="([^"]+)"[^>]*>([\s\S]*?)<\/button>/);
  if (sb) body[sb[1]] = collapse(sb[2].replace(/<[^>]+>/g, " ")) || "";

  const pickupLocation =
    region.match(/id="pickuplocs"[\s\S]*?form-static-text">([^<]+)</)?.[1]?.trim() ?? null;
  const statics = [...region.matchAll(/form-static-text">([^<]+)</g)].map((m) => collapse(m[1]));
  const availableSummary = statics.find((s) => /reservable|available/i.test(s)) ?? null;

  const expiryChoices: { days: string; label: string }[] = [];
  const xdays = region.match(/<select\b[^>]*\bname="XDAYS"[^>]*>([\s\S]*?)<\/select>/)?.[1] ?? "";
  for (const o of xdays.matchAll(/<option[^>]*\bvalue="([^"]*)"[^>]*>\s*([^<]*)/g)) {
    expiryChoices.push({ days: o[1], label: collapse(o[2]) });
  }

  return { action, body, pickupLocation, availableSummary, expiryChoices };
}

export function toReserveOptions(form: ReserveForm | null): ReserveOptions {
  if (!form) {
    return {
      canReserve: false,
      pickupLocation: null,
      availableSummary: null,
      expiryChoices: [],
      message: "This title can't be reserved right now.",
    };
  }
  return {
    canReserve: true,
    pickupLocation: form.pickupLocation,
    availableSummary: form.availableSummary,
    expiryChoices: form.expiryChoices,
    message: null,
  };
}

// --- Cancel / edit (manage an existing reservation) -----------------------
//
// Both are CCOPT wizards that share the mainForm shape parsed above:
//   Cancel  GET …/C/5/RSVCENQ?SVL=  → confirm form → POST …/C/6/RSVCENQ
//   Edit    GET …/ER/1/RSVCENQ?SVL= → edit form     → POST …/ER/2/RSVCENQ
// The confirm/edit form carries a `submitButton` (type=submit, "Continue") that
// commits, and a `cancelButton` (type=button, JS nav) that aborts — only the
// former is type=submit, so parseReserveForm never selects the abort button.

export interface EditForm {
  action: string;
  body: Record<string, string>;
  pickupChoices: PickupChoice[];
  pickupSelected: string | null;
  delayUntil: string | null;
  notRequiredAfter: string | null;
}

/** Serialize the edit-reservation `#mainForm` plus its pickup-location choices. */
export function parseEditForm(html: string, origin: string): EditForm | null {
  const base = parseReserveForm(html, origin);
  if (!base) return null;

  const sel = html.match(/<select\b[^>]*\bname="PICKUP"[^>]*>([\s\S]*?)<\/select>/)?.[1] ?? "";
  const pickupChoices: PickupChoice[] = [];
  for (const o of sel.matchAll(/<option[^>]*\bvalue="([^"]*)"[^>]*>\s*([^<]*)/g)) {
    const code = decode(o[1]);
    const name = collapse(o[2]);
    if (code && name) pickupChoices.push({ code, name });
  }
  const pickupSelected =
    sel.match(/<option[^>]*\bselected\b[^>]*\bvalue="([^"]*)"/)?.[1] ??
    sel.match(/<option[^>]*\bvalue="([^"]*)"[^>]*\bselected\b/)?.[1] ??
    null;

  return {
    action: base.action,
    body: base.body,
    pickupChoices,
    pickupSelected: pickupSelected || null,
    delayUntil: base.body["SDTE"] || null,
    notRequiredAfter: base.body["XDTE"] || null,
  };
}

export function toEditOptions(form: EditForm | null): EditOptions {
  if (!form) {
    return {
      canEdit: false,
      pickupChoices: [],
      pickupSelected: null,
      delayUntil: null,
      notRequiredAfter: null,
      message: "This reservation can't be edited right now.",
    };
  }
  return {
    canEdit: true,
    pickupChoices: form.pickupChoices,
    pickupSelected: form.pickupSelected,
    delayUntil: form.delayUntil,
    notRequiredAfter: form.notRequiredAfter,
    message: null,
  };
}

// An explicit cancel refusal: "This reservation cannot be cancelled.", "can't be cancelled",
// "could not cancel", "unable to cancel", "not allowed to cancel", … The negation must sit right
// before the cancel verb (only be/now/currently/yet between), so help text such as "If you cannot
// collect it, you can cancel it" never trips it.
const CANCEL_REFUSAL =
  /\b(?:cannot|can not|can['’]?t|could not|couldn['’]?t|can no longer|unable to|not able to|not allowed to|not permitted to)(?:\s+(?:be|now|currently|yet))*\s+cancel/i;
// The refusal's sentence: bounded by . ! ? or a block-element edge (\u0000), at most 160 chars each side.
const CANCEL_REFUSAL_SENTENCE = new RegExp(
  `[^.!?\\u0000]{0,160}${CANCEL_REFUSAL.source}[^.!?\\u0000]{0,160}[.!?]?`,
  "i",
);
const SCRIPT_STYLE = /<script\b[^>]*>[\s\S]*?<\/script>|<style\b[^>]*>[\s\S]*?<\/style>/gi;
const BLOCK_TAG =
  /<\/?(?:address|article|aside|blockquote|body|br|button|caption|dd|details|div|dl|dt|fieldset|figcaption|figure|footer|form|h[1-6]|header|hr|html|label|legend|li|main|nav|ol|option|p|section|select|summary|table|tbody|td|textarea|tfoot|th|thead|title|tr|ul)\b[^>]*>/gi;

/** The sentence that refuses a cancel, whitespace-collapsed; null when the page has none. */
function cancelRefusal(html: string): string | null {
  // Visible-ish text: scripts/styles dropped, block edges kept as \u0000 so a sentence never
  // runs across elements, inline tags → spaces, entities decoded.
  const text = decode(
    html.replace(SCRIPT_STYLE, " ").replace(BLOCK_TAG, "\u0000").replace(/<[^>]+>/g, " "),
  );
  if (!CANCEL_REFUSAL.test(text)) return null;
  const m = text.match(CANCEL_REFUSAL_SENTENCE);
  return m ? m[0].replace(/\s+/g, " ").trim() : null;
}

/** Classify the page returned after committing a cancel / edit. */
export function parseManageResult(html: string, kind: "cancel" | "edit"): ReserveResult {
  const text = collapse(html.replace(/<[^>]+>/g, " "));
  const err = html.match(/id="formValidationMsg"[^>]*>([^<]{2,160})</)?.[1]?.trim();
  if (err && !/^error$/i.test(err)) return { ok: false, message: collapse(err) };

  if (kind === "cancel") {
    // Checked before the success wording: a refusal also says "reservation" and "cancelled".
    const refusal = cancelRefusal(html);
    if (refusal) return { ok: false, message: refusal };
    if (/cancell?ed/i.test(text) && /reservation|request|hold/i.test(text)) {
      return { ok: true, message: "Reservation cancelled." };
    }
  } else if (/reservation[s]?[^.]{0,40}(updated|changed|amended|saved)/i.test(text) ||
             /successfully (updated|changed|amended)/i.test(text)) {
    return { ok: true, message: "Reservation updated." };
  }

  const fail = text.match(/(cannot|could not|unable to|not allowed|error)[^.]{0,80}/i);
  // Left the wizard (no mainForm) → Spydus redirected back to the list: success.
  if (!/id="mainForm"/.test(html)) {
    return { ok: true, message: kind === "cancel" ? "Reservation cancelled." : "Reservation updated." };
  }
  return {
    ok: false,
    message: fail ? collapse(fail[0]) : kind === "cancel" ? "Couldn't cancel the reservation." : "Couldn't update the reservation.",
  };
}

/** Classify the page returned after POSTing the reservation. */
export function parseReserveResult(html: string): ReserveResult {
  const text = collapse(html.replace(/<[^>]+>/g, " "));
  // A validation error re-renders the form with a populated message.
  const err = html.match(/id="formValidationMsg"[^>]*>([^<]{2,120})</)?.[1]?.trim();
  if (err && !/^error$/i.test(err)) return { ok: false, message: collapse(err) };

  if (/(reservation|request)[^.]{0,40}(placed|successful|added|submitted|received|confirmed)/i.test(text) ||
      /successfully reserved/i.test(text) ||
      /you have (successfully )?reserved/i.test(text)) {
    return { ok: true, message: "Reservation placed." };
  }
  const fail = text.match(/(you already have|already reserved|cannot be reserved|not allowed|unable to|maximum|exceeded|too many)[^.]{0,80}/i);
  if (fail) return { ok: false, message: collapse(fail[0]) };

  // Still on the form → an extra step is required we didn't complete.
  if (/id="mainForm"/.test(html)) {
    return { ok: false, message: "Reservation could not be completed." };
  }
  return { ok: true, message: "Reservation placed." };
}
