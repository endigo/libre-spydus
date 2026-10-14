# Canning OPAC web contracts

Collected 11 October 2026 from [Canning Libraries OPAC](https://canning.spydus.com/cgi-bin/spydus.exe/MSGTRN/OPAC/HOME). This inventory covers public browser contracts and authorised authenticated read-only OPAC requests to support the rebuild investigation. It does **not** establish every native app API contract. The native app's captured requests and managed assembly evidence belong in the main endpoint inventory.

The [machine-readable inventory](web-endpoints.json) contains 36 route/action entries with source snippets (authenticated IDs parameterized), line and column locations, confidence, query/body details, and mutation flags; 33 configured search-category options; and 9 exact lazy-fragment links. Original first-party snapshots and SHA-256 fetch provenance are under `.private/web/public/`. Files there remain private/ignored. The authenticated follow-up used an app-derived session cookie stored privately. Exact requests, response headers and account HTML are in `.private/web/auth/reads.har`; the HAR omits request Cookie values, while response headers/HTML remain private. No reservation, renewal, cancellation, edit, notification-read, review, list change, logout, or authenticator action was submitted.

REA 6.3.0 statically analyzed the downloaded JavaScript/HTML with:

```sh
npx rea-agents@latest analyze-javascript-application \
  .private/web/public --artifact-format directory --format json
```

The sealed result is `.private/web/rea-public-js.json`, evidence ID `ev_f97fd26a718f07d2f8f56c251b2d1857434ee5afe5b0878e7e072008930a24cc`. REA parsed inert text and did not execute the bundles. A source reference confirms client behavior, not successful server execution.

## Catalogue reads

All paths use `https://canning.spydus.com`. CGI paths below have prefix `/cgi-bin/spydus.exe`. Canning's `body[data-subsite]` is empty; the scripts support a prefix for other deployments. `body[data-instcode]` is `WQUP`.

| Feature | Method and path | Request/response evidence |
| --- | --- | --- |
| Keyword search | `GET /ENQ/OPAC/BIBENQ` | Public GET accepted. `ENTRY`, `ENTRY_NAME=BS`, `ENTRY_TYPE=K`, `SORTS=SQL_REL_BIB`, `GQ` → HTML results. Header form also supplies `NRECS=10`, `QRY`, `QRYTEXT`, `_SPQ=2`. |
| Grouped work search | `GET /ENQ/OPAC/BIBWRKENQ` | Configured header option: `SORTS=SQL_REL_WRK`, `CF=BIB`. Referenced but not called. |
| Cloud search | `GET /CLDENQ/{tpc}` | Conditional script branch, only when chosen option has `data-cloud=1`; active Canning option not established. |
| Result pagination | `GET /SET/OPAC/BIBENQ/{setId}/{cursor}?NREC={offset}` | Returned result links contain `NREC=0` and `10`; cursor can be `0` or a record IRN. Follow returned links. |
| Record detail | `GET /FULL/OPAC/BIBENQ/{setId}/{bibIrn},{ordinal}` | Public GET accepted; full HTML record, reservation links, related content. |
| Availability | `GET /XHLD/OPAC/BIBENQ/{setId}/{bibIrn}?RECDISP=REC` | Public GET accepted; HTML table containing location, collection, call number, status/description. Query optional. |
| Record fragment | `GET /XFULL/OPAC/BIBENQ/{setId}/{bibIrn}?FMT=REC` | Nested fragment link in availability HTML; not fetched. |
| Facets | `GET /XFACETS/OPAC/BIBENQ/{setId}` | Hidden result-page link loaded by JavaScript; not fetched. |
| Home carousel | `GET /RUNSQRY/OPAC/BIBENQ` | Saved query ID `SQRYIRN`; `FMT=GD`, `XSLT=XGridView.xsl`, `SORTS=SQL_RANDOM`, `NRECS`, `SETLVL=SET`, `GCN=BESTSELLERS`, `ISGLB=0`. Referenced only. |
| Suggestions | `POST /autocompleter/GetFilteredTerms` | JSON `{ "mode": "QF", "value": "<search text>", "inst": "WQUP" }`; client inserts response as HTML list items. Referenced only. |

Search category filters are Spydus query-language strings rather than client-side enums. Examples from the public page include `BIBRECFMT: BK`, `BIBRECFMT: EBK`, `BIBRECFMT: EAUD`, `BIBLOCX: LH` for work search, with different query strings for bibliographic search. The JSON inventory preserves the complete observed option attributes.

Sources: [spydus.js](https://canning.spydus.com/docs/OPAC/js/spydus.js) lines 48–84 (header form action), [searchAutoComplete.js](https://canning.spydus.com/docs/OPAC/js/searchAutoComplete.js) lines 58–80 (request), [tabContainer.js](https://canning.spydus.com/docs/OPAC/js/tabContainer.js) lines 37–63 (fragment loading), and local `search.html`, `detail.html`, `holdings.html` snapshots. HTML is mostly one long source line; JSON locations include columns and exact attribute values for reliable lookup.

The record-detail page also supplies lazy URLs for tags, reviews, author titles, similar titles and “people who borrowed this also borrowed.” Those use the same CGI query system with `QRY`, `XSLT`, `FMT`, `SORTS`, `SETLVL`, `NRECS` and repeated `EFT` parameters. The last label is a **public recommendation query**, not the signed-in patron's loans. Exact URLs and selectors are in `exact_public_html_fragment_links` in the JSON.

## Browser account and session behavior

`GET /MSGTRN/OPAC/LOGINB` returns a login form. The guest dashboard `GET /PGM/OPAC/CCOPT/LB/1?ISGLB=0` also succeeds with HTML, but does not reveal authenticated loan/reservation cards.

The browser login form submits `POST /PGM/OPAC/CCOPT/LB/2` with fields `BRWLID`, `BRWLPWD` and `RDT`. The form's default encoding implies `application/x-www-form-urlencoded`. Before submission, `BRWLPWD` is encrypted with JSEncrypt using the public RSA key shipped in `encrypt.js`; ciphertext is inserted into the password field. Both the navbar `#frmLogin` and page `#mainForm` have handlers. Login was not submitted, so session issuance, error responses, MFA transitions and credential compatibility with the native app are unverified.

`spydus.js` lines 106–112 inject a hidden `csrf` field into POST forms when `body[data-csrf]` exists. Captured guest pages do not have that attribute. Absence in a guest page does not establish that authenticated requests omit CSRF.

The scripts make same-origin AJAX requests without an explicitly assigned Authorization header. The initial public evidence did not include authenticated cookies. The follow-up authenticated reads establish acceptance of the app-derived session cookie, while original WebView request headers and browser-cookie issuance remain unverified. `SetActiveSavedListCookie` uses `P` plus the cookie named by `GetCookie("NAME")`, falling back to `BRWSESSION`; this is a **saved-list cookie convention**, not proof of the authentication cookie name. Opaque query/result `setId` values are embedded in subsequent links; their lifetime and cookie binding are unknown.

Authenticated dashboard cards are loaded using `.js-tab-container-brw-dashboard-item`, which reads a runtime `#url{id}` link and loads its HTML into the card. The initial public evidence did not supply exact authenticated routes. The authenticated follow-up below verifies this deployment’s account, current loans, pickup reservations and pending reservations routes.

`GET /PGM/OPAC/CCOPT/LB/3?RDT=<return URL>` is the logout menu action and was not followed.

Sources: [login.js](https://canning.spydus.com/docs/OPAC/js/login.js), [form.js](https://canning.spydus.com/docs/OPAC/js/form.js), [encrypt.js](https://canning.spydus.com/docs/OPAC/js/encrypt.js), `spydus.js` lines 106–112 and 179–184, and `tabContainer.js` lines 51–63.

## Authenticated reads and action references

The app-derived `SPYDUS_BRWSESSIONID_443` cookie was accepted for authenticated account, loan and reservation reads. Its value is omitted from documentation. Native method `0x060007d6` assigns these UI cookies: `SPYDUS_DISPLAYCONTEXT_443=MOB`, `consentSettings_443=ALL`, and `SPYDUS_MYACCOUNT_443` with an **empty** value (`System.String.Empty`, CIL offset 645, MemberRef `0x0A000195`). A separate private cookie jar with those UI values also produced authenticated HTML.

The Canning account entry `/PGM/OPAC/CCOPT/LB/1?&RFB=1` returns HTTP 200 with a meta refresh to `/ENQ/OPAC/BRWENQ/{setId}?QRY=%23{setId}&QRYTEXT=My%20details&NRECS=999`. The earlier entry using `/PGM/WQUP/…` returned an `/ENQ/WQUP/BRWENQ/…` target, which produced HTTP 400; changing that target to `/ENQ/OPAC/BRWENQ/…` succeeded. This is an observed Canning context correction, not a rule established for other tenants. It demonstrates that `WQUP` institution code and the default `OPAC` path context must remain separate.

| Coverage | Route after CGI prefix | Fields/evidence |
| --- | --- | --- |
| **Authenticated GET verified** | `/ENQ/OPAC/BRWENQ/{setId}` | Account HTML and navigation; `QRY`, `QRYTEXT`, `NRECS`. Authenticated body includes `data-csrf`. |
| **Authenticated GET verified** | `/ENQ/OPAC/LOANRENQ/{setId}` | `FMT=CL`; `QRY` scopes patron/current-loan status; `SETLVL=SET`, `NRECS=30`. Loan HTML includes due/status and action links. |
| **Authenticated GET verified** | `/ENQ/OPAC/RSVCENQ/{setId}` | `FMT=AR` identifies available-for-pickup list; `FMT=WR` identifies pending list. `QRY`, `SORTS`, `SETLVL`, `NRECS`. |
| **Static mutation reference only** | `/PGM/OPAC/CCOPT/{setId}/0/N/3/LOANRENQ` | Renew one/selected loans: `SVL={loanIrn}`, `SETDESC=Current loans`; selected form also submits `NREC`. |
| **Static mutation reference only** | `/PGM/OPAC/CCOPT/{setId}/0/N/2/LOANRENQ` | “Renew all” toolbar action. |
| **Static mutation reference only** | `/PGM/OPAC/CCOPT/{setId}/0/C/5/RSVCENQ` | Cancel one/selected reservations: `SVL={reservationIrn}`, `SETDESC`; selection form includes `NREC`. |
| **Static mutation reference only** | `/PGM/OPAC/CCOPT/{setId}/0/C/4/RSVCENQ` | “Cancel all” toolbar action. |
| **Static mutation reference only** | `/PGM/OPAC/CCOPT/{setId}/0/ER/1/RSVCENQ` | Edit one/selected reservations. |

The authenticated list form `#frmSVL` uses GET. `SubmitSetoptsAction` (`spydus.js` lines 382–393) sets its action and submits checked `SVL` inputs plus hidden `SETDESC` and `NREC` fields. These action links were inspected statically; opening them might perform an action, so final confirmation forms, eligibility checks and success/error responses remain unobserved. The presence of `body[data-csrf]` means POST forms can receive CSRF; enforcement for these GET actions has not been measured.

The follow-up retained **14 private read captures**, including failed `WQUP` context probes and successful OPAC reads. The 36-entry JSON inventory contains **4 additional authenticated read contracts and 5 additional static mutation references**, alongside the original 27 public entries. Account-entry behavior is attached to the existing dashboard entry.

### HTML field mapping

[account-shape.json](account-shape.json) records selectors and extraction types without member names, barcodes, reading titles, dates, counts, identifiers or tokens. The observed account dashboard uses `.brw-dashboard-item` anchors with `.brw-dashboard-item-header`, `.brw-dashboard-item-body` and `.brw-dashboard-item-footer`; preserve the anchor href as a read navigation target.

Loan/reservation rows use `#mainContent .result-content-records table.table-stacked-image tbody > tr`. `input[name=SVL]` inside `td.counter-select` carries the **loan/reservation IRN**, while `h3.card-title a` supplies title and record URL. These IDs must not be confused with the bibliographic record ID used by catalogue APIs. Cover candidates are in `img.imgsc[data-imgurls]`, separated by `**`; metadata lines appear in `.card-text > span.d-block` and currently lack verified per-field labels.

Current-loan due text is `td[data-caption="Due"]`; status is the fifth cell. Pickup-reservation status and pickup-location links live in `td[data-caption="Status"]`. Pending reservations additionally have `td[data-caption="Placed"]`, `td[data-caption="Rank"]` and a separate expiry cell; desktop markup duplicates placed/expiry data in a combined dates cell. Deduplicate responsive mobile/desktop fields and repeated action menus. These selectors describe observed Canning HTML, not a guaranteed cross-tenant API or stable schema.

## Reservations and list actions — static only

The full record and availability HTML provide a “Place reservation” link beginning:

```text
GET /cgi-bin/spydus.exe/PGM/OPAC/CCOPT/{setId}/{bibIrn}/V
```

Its `FRMTYP=R` and nested `RDT` return target lead through another `/V` route to `/CCOPT/{setId}/{bibIrn}/R`. `modal.js` assigns this exact supplied URL to `window.location.href` on reservation-button activation. The search result also provides a selection path:

```text
GET /cgi-bin/spydus.exe/PGM/OPAC/CCOPT/{setId}/0/V/0/R/9/BIBENQ
    ?SVL={bibIrn}&RECFMT=
```

These are workflow entry references. The roles of `V`/`R`, subsequent form fields, pickup locations, server validation and final confirmation are unverified. They were not opened or replayed.

Pick-list changes also use GET: `/XSVL/OPAC/BIBENQ/{setId}?SVL={bibIrn}` adds and `?RSVL={bibIrn}` removes. `onClickSVL` expects XML and inspects `REDIRECT` and `COOKIE` (`NAME`/`VALUE`) nodes. That shows why method alone cannot determine whether a request is read-only. List modal POST actions are supplied by the triggering HTML links and are not hardcoded in `modal.js`.

Sources: local `detail.html`, `search.html`, `holdings.html`; [modal.js](https://canning.spydus.com/docs/OPAC/js/modal.js) lines 59–85 and 211–240; `spydus.js` lines 867–889.

## Additional web APIs — static only

| Action | Route | Client expectation |
| --- | --- | --- |
| Read public review totals | `GET /api/open/1.0/general/quickreview/{bibIrn}` | Object with `like` and `dislike` counts. |
| Read patron quick review | `GET /api/wpac/1.0/patrons/{brwIrn}/quickreview/{bibIrn}` | Boolean `true` means like; `false` means dislike; other value leaves no selection. |
| Set/remove quick review | `GET` same patron route plus `/like`, `/dislike`, `/clear` | Boolean `true` triggers UI success. Mutates patron data. |
| Read authenticator state | `GET /api/wpac/1.0/patrons/{brwIrn}/authapp` | Text secret key; 404 treated as disabled. |
| Enable/reset authenticator | `PUT` same `/authapp` route | No explicit body/headers in this script; response text secret key. Mutates security state. |
| Disable authenticator | `DELETE` same `/authapp` route | Successful status clears UI. Mutates security state. |
| Generate QR image | `GET /api/open/1.0/qrcodes?id={encodedContent}` | Image src; authenticator UI passes an encoded `otpauth://` URI. |

Sources: `spydus.js` lines 893–1040 and `modal.js` lines 248–350. None of these APIs was called. Their presence in the OPAC scripts does not prove the mobile app uses them or that Canning enables every feature.

## Client implications and remaining evidence

Public catalogue HTML can inform screen/data mapping, but use native app API evidence to select the client's transport and model contracts. Keep catalogue search, record ID, work ID, result set ID and patron ID distinct. Preserve server-provided workflow links and query strings if implementing an OPAC compatibility layer. Avoid calling any GET action automatically on page load: reservation, list, quick review and logout links can change state.

Authenticated Canning account, current-loan, pickup-reservation and pending-reservation HTML reads are now verified. Remaining gaps include charges/history, account notification-read semantics, renewal/cancellation/edit eligibility and success/error responses, reservation creation pickup/confirmation, expiry behavior and native JSON counterparts. Those need further managed-code evidence or authorised request observations. This inventory makes no claim to cover every endpoint.
