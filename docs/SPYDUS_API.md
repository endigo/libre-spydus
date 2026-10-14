# Spydus Library — Reverse‑Engineered API Specification

> Source: Civica **Spydus Library** Android app `com.civica.civicaspydus` v2.0.6 (build 784),
> extracted from the APK installed on the `Pixel_10_Pro` emulator.
> App type: **.NET MAUI** (C#, AOT‑compiled), HTTP layer built on **Refit**.
> This document is the complete API contract recovered from the app's own typed REST
> client interfaces, DTOs and OAuth config — intended for building an interoperable
> client (LibreSpydus).

---

## 1. Architecture overview

The app talks to **three distinct backends**:

| # | Host | Purpose | Auth | Protection |
|---|------|---------|------|-----------|
| 1 | `https://librarylist.spydus.com/api/1.0` | Global library directory + app languages | none (public) | open (confirmed live) |
| 2 | `https://idp.spydus.com` | OpenID/OAuth2 identity provider (login, token, logout) | OAuth2 auth‑code | — |
| 3 | `https://<library>.spydus.*` (per library, e.g. `https://aberdeencity.spydus.co.uk`) | All catalogue/patron/config endpoints + classic OPAC web views | Bearer JWT + OPAC session cookie | **AWS WAF** (bot/human challenge), **TLS certificate pinning** |

The per‑library server base URL is **discovered at runtime**: the app calls host #1 to
list libraries, the user picks one, and `LibraryDto.url` becomes the base address for all
host #3 calls. A per‑library `HttpClient` is created with a `CertificatePinningHandler` and
a 30 s timeout (`DynamicApiClientFactory`).

### App config (from `assets/app.config`)

```
AuthenticationUrl      = https://idp.spydus.com
LibraryListApiUrl      = https://librarylist.spydus.com/api/1.0
ClientId               = spydusmobile
CallbackUrl (redirect) = spyduslibrary://auth        (Android deep link: scheme=spyduslibrary host=auth)
Customer               = SpydusLibrary
DefaultLanguage        = ENG
Scope                  = openid profile mobile        (from AuthConstants)
GroupCode              = (empty)
SpydusPrimaryPinningHash = 2BWWoN1scd+yGvhqGTiPnKZsxSHfv6FIMFKs2MLmA3A=   (SPKI SHA‑256)
SpydusBackupPinningHash  = PmYXBbYmXsWCxmSRUgnhfYmJ09ETroEOgIITe4AFntI=   (SPKI SHA‑256)
```

Default per‑library pins above are overridden per library by `LibraryDto.pinningHash`
when the directory supplies them.

---

## 2. Authentication (host #2 — `idp.spydus.com`)

OAuth2 **Authorization Code** flow (no PKCE observed), driven through a system browser /
`WebAuthenticator`, with the custom scheme redirect `spyduslibrary://auth`.

### 2.1 Authorize (interactive, opens in browser)
```
GET https://idp.spydus.com/openid/authorize
      ?client_id=spydusmobile
      &redirect_uri=spyduslibrary://auth
      &response_type=code
      &scope=openid profile mobile
      [&prompt=login]        # added when a forced interactive re-login is requested
```
Result: redirect to `spyduslibrary://auth?code=<authorizationCode>`.

### 2.2 Exchange code for tokens
```
GET /openid/token?code={authorizationCode}&grant_type=authorization_code&client_id=spydusmobile
→ 200 { "access_token": "...", "refresh_token": "..." }     # RefreshTokenResponse
```

### 2.3 Refresh access token
```
GET /openid/refreshtoken?refreshToken={refreshToken}&clientId=spydusmobile
→ 200 { "accessToken": "...", "refreshToken": "..." }        # AccessTokenResponse  (note camelCase differs from 2.2)
```

### 2.4 Logout
```
POST /openid/patronlogout?refreshToken={refreshToken}&clientId=spydusmobile
→ 200
```

### 2.5 Access token (JWT) claims used by the app  (`ClaimKeys`)
| Claim | Meaning |
|-------|---------|
| `BrwId` | Borrower barcode |
| `BrwdIrn` | Borrower consortium record identifier (used in notification paths) |
| `BrwIrn` | Borrower local account identifier (used in device‑token paths) |
| `BrwSessionId` | Borrower OPAC session id |
| `InstitutionId` | Institution id |
| `client_id`, `name`, `preferred_username`, `ver` | standard |

### 2.6 OPAC session reconciliation
Login also establishes a **classic Spydus OPAC session** in a WebView. Cookies observed:
`SPYDUS_BRWSESSIONID_443`, `SPYDUS_DISPLAYCONTEXT_443`, `SPYDUS_MYACCOUNT_443`.
The OPAC login form is reached at `.../cgi-bin/spydus.exe/.../CCOPT/LB/16`
(`StandardOpacLoginFormIndicator = "/CCOPT/LB/16"`). The `BrwSessionId` is then validated
against the API (see `POST /api/private/1.0/patrons/validatesessionid`).

### 2.7 Bearer header
Refit `[Authorize("Bearer")]` params emit a standard `Authorization: Bearer <accessToken>`.
A `DelegatingHandler` injects the token and refreshes on expiry via `IAuthenticationService`.
> Note: `ISelfIssueClient` is annotated with a (misspelled) class‑level header
> `Authorisation: Bearer` in addition — reproduce exactly if targeting that endpoint.

---

## 3. Library directory & languages (host #1 — `librarylist.spydus.com/api/1.0`)

### 3.1 List libraries  ✅ live‑confirmed (351 entries)
```
GET /mobile/libraries?groupCode={groupCode}      # groupCode optional/empty
→ 200 LibraryDto[]
```
```json
{
  "name": "Aberdeen City Libraries",
  "id": "62fc2a6d-ca77-4ed9-b77e-417f5fa5b10f",
  "url": "https://aberdeencity.spydus.co.uk",
  "logo": null,
  "activationCode": null,
  "location": { "country": "UK", "state": "Scotland", "latitude": 57.14786, "longitude": -2.10619 },
  "pinningHash": null,
  "groupCode": null
}
```

### 3.2 List app languages
```
GET /mobile/languages?groupCode={groupCode}
→ 200 AppLanguageDto[]        # { "iso-code", "language-pac", "name" }
```

---

## 4. Per‑library API (host #3 — base = `LibraryDto.url`)

Path prefixes: `/api/open/1.0` (public), `/api/mobile/1.0` (mobile), `/api/private/1.0` (internal).
Common header: `spydus-lang: <languageId>` (e.g. `ENG`). Bearer where noted.

### 4.1 Server / config / branding

| Method | Path | Auth | Returns | Notes |
|--------|------|------|---------|-------|
| GET | `/api/open/1.0/general/version` | none | `ServerInfo` | `{ client, release, license, server, patch }` |
| GET | `/api/mobile/1.0/general/config?wifiName={wifiName}` | none | `string` (JSON → `StyleConfigDto`) | app styling + feature flags |
| GET | `/api/mobile/1.0/general/logo` | none | `string` | logo (url or base64) |
| GET | `/api/mobile/1.0/containers?wifiName={wifiName}` | none | `ContainerResponse` | home‑screen tiles; header `spydus-lang` |
| GET | `/api/mobile/1.0/announcements` | none | `AnnouncementResponseDto` | header `spydus-lang` |
| GET | `/api/mobile/1.0/mobile/branches?setId={setId}&lastIrn={lastIrn}&count={count}` | none | `BranchResponseDto` | paged; header `spydus-lang` |

### 4.2 Discovery / search
| Method | Path | Auth | Returns |
|--------|------|------|---------|
| GET | `/api/mobile/1.0/discover/search?{searchMode}={searchData}` | none | `string` |

`searchMode` ∈ { `keywords`, `barcodeData` } (`DiscoverSearchModeEnum`). Header `spydus-lang`.
Returns a raw string (OPAC discover payload/URL for the results WebView).

### 4.3 Patron notifications
| Method | Path | Auth | Body | Returns |
|--------|------|------|------|---------|
| GET | `/api/mobile/1.0/patrons/{brwdIrn}/notifications` | Bearer | — | `List<NotificationDto>` (`NotificationResponseDto`) |
| PATCH | `/api/mobile/1.0/patrons/{brwdIrn}/notifications/{id}/{status}` | Bearer | — (`Content-Type: application/json; charset=UTF-8`) | `bool` |
| POST | `/api/mobile/1.0/patrons/{patronIrn}/notifications/device` | Bearer | `RegisterDeviceTokenRequest` | — |
| PUT | `/api/mobile/1.0/patrons/{patronIrn}/notifications/device` | Bearer | `UpdateDeviceTokenRequest` | — |
| DELETE | `/api/mobile/1.0/{patronIrn}/notifications/device/{deviceToken}` | Bearer | — | — |

`{brwdIrn}` = `BrwdIrn` claim (consortium id); `{patronIrn}` = `BrwIrn` claim (local id).

### 4.4 Self‑service (issue / return) — `ISelfIssueClient`  (class header `Authorisation: Bearer`)
| Method | Path | Returns |
|--------|------|---------|
| POST | `/api/mobile/1.0/patrons/{patronIrn}/items/{itemId}/issue?alert_acknowledged={bool}` | `SelfIssueResponseDto` |
| POST | `/api/mobile/1.0/patrons/{patronIrn}/items/rfid/{itemRfid}/issue?alert_acknowledged={bool}` | `SelfIssueResponseDto` |
| POST | `/api/mobile/1.0/patrons/{patronIrn}/items/{itemId}/return?alert_acknowledged={bool}` | `SelfIssueResponseDto` |

All `Content-Type: application/json; charset=UTF-8`.

### 4.5 Membership card (wallet)
| Method | Path | Auth | Returns |
|--------|------|------|---------|
| GET | `/api/mobile/1.0/patrons/{patronId}/wallet/ios` | Bearer | binary (`.pkpass`) |
| GET | `/api/mobile/1.0/patrons/{patronId}/wallet/google?language={isoCode}` | Bearer | `string` (Google Wallet save URL) |

### 4.6 Session validation (internal)
| Method | Path | Body | Returns |
|--------|------|------|---------|
| POST | `/api/private/1.0/patrons/validatesessionid` | `ValidateSessionRequest { sessionId }` | `ValidateSessionResponse` |

`Content-Type: application/json; charset=UTF-8`.
`ValidateSessionResponse = { institutionId, name, barcode, brwIrn, brwdIrn, brwSessionId }`.

### 4.7 Classic OPAC web views (loaded in a WebView, not JSON APIs)
| Purpose | Path template (`ApiPathConstants`) |
|---------|-------------------------------------|
| My Account | `/cgi-bin/spydus.exe/PGM/{opacProgram}/CCOPT/LB/1?&RFB=1` |
| OPAC root | `/OPAC/` |
| Join online | `/cgi-bin/spydus.exe/MSGTRN/{opacProgram}/JOINSEL` |
| OPAC login form | `.../CCOPT/LB/16` |

---

## 5. Data models (DTOs)

Request/response models are in `reference/Spydus.Domain.DTO/` (decompiled C#). Highlights:

- **StyleConfigDto** — feature flags & theming: `barcodeType`, `branchUrl`, `contentContainerUrl`,
  `langs[]`, `logoUrl`, `notificationHubConnectionString`, `notificationHubName`,
  `selfIssueEnabled`, `selfReturnEnabled`, `enableJoinOnline`, `spydusViewOverride`,
  `style{}` (string map), `wifi{networks[]}`, `discovery{searchType,defaultSort,advancedSearchUrl}`,
  `joinOnlineIcon`, `distanceUnit`, `libraryAppTimeZone`, `showNearbyBranches`.
- **ContainerDto** — home tiles: `title`, `loginRequired` (0/1/2), `icon`, `url`, `type`.
- **BranchDto** — `irn`, `id`, `name`, `latitude`, `longitude`, `url`, `opacUrl`, `imageUrl`,
  `phone`, `email`, `address{...}`, `openingHours[]`, `holidays[]`; paged via `SetInfo`.
- **NotificationDto** — `brwdIrn`, `brwIrn`, `createdDate`, `ntfId`, `instCode`, `isRead`,
  `libraryId`, `message`, `subject`, `url`.
- **SelfIssueResponseDto** — `alerts[]`, bib metadata, `dueDate`, `isIssued`, `itemCheckedIn`,
  `itemId/itemIrn/itemRfid`, `loanIrn`, `patronIrn`, `rentalAmount`.
- **AnnouncementDto**, **AppLanguageDto**, **LanguageDto**, **LibraryDto**, **LocationDto**,
  **ServerInfo**, **RegisterDeviceTokenRequest**, **UpdateDeviceTokenRequest**,
  **ValidateSessionRequest/Response**, **Wifi/Network** — see `reference/`.

Push notifications use **Azure Notification Hubs** (connection string + hub name come from
`StyleConfigDto`); device registration via §4.3.

---

## 6. Local persistence (on‑device, for parity)

EF Core + **encrypted SQLite** (`sqlite3mc`). Tables (from `Spydus.DAL`): `Account`,
`Library`, `Branch`, `Notification`, `StyleConfig`, `ServerInfo`, `ActivationCode`, `LogEvent`.
`Account` stores `AccessToken`, `RefreshToken`, `BorrowerId`, `BorrowerSessionId`, `ClientId`,
`InstitutionId`, `LibraryId`, `Name`, `PreferredUsername`, `Version`.

---

## 7. Notes for the rebuild

- **AWS WAF** guards per‑library servers; a plain HTTP client gets a `Human Verification`
  challenge page. The app passes because requests originate from the same WebView/session
  that solved the WAF challenge (AWS WAF token cookie). A rebuilt client will likely need a
  hidden WebView to acquire the `aws-waf-token` cookie and attach it to API calls.
- **Certificate pinning**: SPKI SHA‑256 pins per library (defaults in app.config). Replicate
  with a pinning HTTP client, or accept the trade‑off for a clone.
- Two token response casings differ (`access_token`/`refresh_token` vs `accessToken`/`refreshToken`).
- Search & My Account are **WebView‑driven** against the classic OPAC, not pure JSON APIs.

*Recovered via static analysis of the shipped APK for interoperability. No app protections
were bypassed; only the app's own public discovery endpoints were contacted (read‑only).*
