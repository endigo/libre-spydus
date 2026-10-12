// JSON shapes the middleware emits. These are the clean contract the Flutter app
// consumes — a faithful projection of the Spydus OPAC HTML, nothing more.

export interface SearchResults {
  query: string | null;
  setId: string | null;
  totalResults: number | null;
  showing: { from: number; to: number } | null;
  /** Absolute URL of the next page of results, or null on the last page. */
  nextPageUrl: string | null;
  /** Absolute URL of the previous page, or null on the first page. */
  prevPageUrl: string | null;
  /**
   * True only for the OPAC's genuine "no records found" page ("Sorry, your search for … did not
   * find any records."); records is then empty. Any other page without records stays false.
   */
  noResults: boolean;
  records: SearchRecord[];
}

export interface SearchRecord {
  rank: number | null;
  /** Work IRN (the SVL checkbox value); identifies the grouped work. */
  workIrn: string | null;
  title: string | null;
  /** Full-record page, e.g. /cgi-bin/spydus.exe/FULL/OPAC/ALLWRKENQ/{setId}/{irn},{ord} */
  detailUrl: string | null;
  author: string | null;
  authorRole: string | null;
  authorUrl: string | null;
  year: string | null;
  language: string | null;
  summary: string | null;
  isbn: string | null;
  coverUrls: string[];
  formats: RecordFormat[];
}

export interface RecordFormat {
  /** e.g. BK, EBK, AB (audiobook) — parsed from the tab element id. */
  code: string;
  label: string | null;
  /** Reservation launch URL (/PGM/OPAC/CCOPT/...). */
  reserveUrl: string | null;
  /**
   * The per-record availability fragment the WebView lazy-loads for this format
   * (/ENQ/OPAC/BIBENQ/{setId}?...RECFMT={code}). Feed this to parseAvailability.
   */
  availabilityUrl: string | null;
}

/** One row in a patron list (a current loan or a reservation). */
export interface PatronItem {
  /** The SVL value — loan IRN or reservation IRN (used by renew/cancel links). */
  irn: string | null;
  /** The bib-record IRN (parsed from bibUrl) — matches a RecordDetail.bibIrn. */
  bibIrn: string | null;
  title: string | null;
  bibUrl: string | null;
  author: string | null;
  year: string | null;
  coverUrls: string[];
  isbn: string | null;
  /** Columnar cells keyed by their `data-caption` (e.g. {Due: "27 Oct 2026"}). */
  fields: Record<string, string>;
  /** Row action links (e.g. Renew loan, Cancel reservation). */
  actions: { label: string; href: string }[];
}

export interface PatronList {
  total: number | null;
  items: PatronItem[];
}

/** Links extracted from the authenticated account (BRWENQ) page. */
export interface AccountLinks {
  name: string | null;
  loansUrl: string | null;
  reservationsReadyUrl: string | null;
  reservationsPendingUrl: string | null;
}

/** One value within a record-detail row — text plus an optional OPAC link. */
export interface DetailValue {
  text: string;
  /** Absolute OPAC URL when the value is a link (author, subject, series…). */
  url: string | null;
}

/** One labelled row of the BIBENQ "Record details" block. */
export interface DetailRow {
  label: string;
  values: DetailValue[];
}

/** The rich bib-record page (BIBENQ) → JSON, mirroring the OPAC "Record details". */
export interface RecordDetail {
  /** The concrete bib-record IRN (matches a reservation's bib IRN). */
  bibIrn: string | null;
  /** Stable bibliographic record number (BRN) — the durable id for this edition. */
  brn: string | null;
  title: string | null;
  /** Series title text (e.g. "Baby-sitters Club (Graphix (Firm))"). */
  seriesTitle: string | null;
  isbn: string | null;
  coverUrls: string[];
  /** Annotation/summary blurb, when present. */
  summary: string | null;
  /** Every labelled row, in page order — render faithfully like the OPAC. */
  details: DetailRow[];
  /** Bookmark (permalink) URL — /ENQ/OPAC/BIBENQ?SETLVL=&BRN={brn}. */
  bookmarkUrl: string | null;
  /** Lazy related-title panels (fetch via /related?url=). */
  related: {
    similar: string | null;
    author: string | null;
    series: string | null;
    versions: string | null;
  };
}

/** A brief card from a related-titles fragment (similar / author / series). */
export interface RelatedItem {
  title: string | null;
  bibUrl: string | null;
  brn: string | null;
  author: string | null;
  year: string | null;
  isbn: string | null;
  coverUrls: string[];
}

/** What the app shows before confirming a reservation (from the OPAC form). */
export interface ReserveOptions {
  /** True when the reservation form was reached (i.e. placing is possible). */
  canReserve: boolean;
  /** Default pickup location name (e.g. "RIVERTON LIBRARY"). */
  pickupLocation: string | null;
  /** Info line, e.g. "Canning - Reservable items 3, available 0". */
  availableSummary: string | null;
  /** "Not required after" choices. */
  expiryChoices: { days: string; label: string }[];
  /** A blocking message when canReserve is false (e.g. login required). */
  message: string | null;
}

/** Outcome of a place / cancel / edit reservation attempt. */
export interface ReserveResult {
  ok: boolean;
  message: string | null;
}

/** A pickup-location choice from the edit-reservation form. */
export interface PickupChoice {
  /** Location code submitted as PICKUP (e.g. "25363"). */
  code: string;
  /** Human name (e.g. "RIVERTON LIBRARY"). */
  name: string;
}

/** What the app shows before editing a reservation (from the OPAC edit form). */
export interface EditOptions {
  /** True when the edit form was reached. */
  canEdit: boolean;
  /** Selectable pickup locations. */
  pickupChoices: PickupChoice[];
  /** Currently selected pickup code, if the form pre-selects one. */
  pickupSelected: string | null;
  /** "Delay until" date already set (yyyy-mm-dd), or null. */
  delayUntil: string | null;
  /** "Not required after" date already set (yyyy-mm-dd), or null. */
  notRequiredAfter: string | null;
  /** Blocking message when canEdit is false. */
  message: string | null;
}

export interface Availability {
  /** The concrete bib-record IRN this format resolves to (differs from workIrn). */
  bibRecordIrn: string | null;
  title: string | null;
  publisher: string | null;
  /** All label:value availability pairs, lower-cased keys (reserved, copies, ...). */
  counts: Record<string, number>;
  reserved: number | null;
  copies: number | null;
  /** Branch-level holdings modal (/XHLD/OPAC/BIBENQ/{setId}/{bibIrn}). */
  holdingsUrl: string | null;
  /** Full record fragment (/XFULL/OPAC/BIBENQ/{setId}/{bibIrn}). */
  recordUrl: string | null;
}
