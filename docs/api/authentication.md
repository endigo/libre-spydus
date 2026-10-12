# Authentication and the OPAC session bridge

This record combines compiled declarations with successful existing-session observations. A fresh login, token exchange, refresh, and logout were not performed: the investigation reused the account already signed in on Pixel_10_Pro.

## Identity settings recovered from the APK

| Setting | Value | Evidence |
| --- | --- | --- |
| Identity base | `https://idp.spydus.com` | `assets/app.config`, `AuthenticationUrl` |
| Client ID | `spydusmobile` | `assets/app.config`, `ClientId` |
| Callback URI | `spyduslibrary://auth` | `assets/app.config`, `CallbackUrl` |
| Authorization route | `/openid/authorize` | `Spydus.Helpers.AuthConstants.OpendIdpPath` |
| Response type | `code` | `AuthConstants.ResponseType` |
| Base scope | `openid profile mobile` | `AuthConstants.Scope` |
| Config default language | `ENG` | `assets/app.config`; this is not the captured Canning header value |

The compiled `OAuthUriBuilderExtensions` maps `client_id`, `redirect_uri`, `response_type`, and `scope`. Its conditional `AddForceInteractive` adds `prompt=login`. Method `0x06000adf` builds scopes containing `libid_` and `lang_` prefixes in addition to the base scope. Those are app-specific scope components; preserve the selected library/language configuration rather than inventing standard OIDC replacements. PKCE/state/nonce handling and their server requirements remain unverified.

## Token endpoints have different wire naming

These are exact Refit declarations in `Spydus.Domain.dll`, not successful captures of token requests.

| Step | Request | Declared response fields |
| --- | --- | --- |
| Exchange authorization code | `GET /openid/token?code={code}&grant_type=authorization_code&client_id=spydusmobile` | `access_token`, `refresh_token` |
| Refresh access token | `GET /openid/refreshtoken?refreshToken={refreshToken}&clientId=spydusmobile` | `accessToken`, `refreshToken` |
| Logout | `POST /openid/patronlogout?refreshToken={refreshToken}&clientId=spydusmobile` | `IApiResponse`; no typed JSON body declared |

The client method names are counterintuitive: `GetRefreshToken` performs the code exchange and `GetAccessToken` calls the refresh endpoint. The compiled exchange call uses `authorization_code` (Spydus.dll token `0x06000ae7`, decoded string at IL 383). Query placement for unannotated scalar parameters follows the Refit client model and has not been captured for these three requests.

Token parsing references the JWT claims `BrwId`, `BrwdIrn`, `BrwIrn`, `BrwSessionId`, `client_id`, `InstitutionId`, `name`, `preferred_username`, and `ver`. Claim names and expired-token checks are recovered from the client, not a promise about every server token version.

## Existing session validation — observed

The original Android app sent:

```http
POST /api/private/1.0/patrons/validatesessionid
Host: canning.spydus.com
Content-Type: application/json; charset=UTF-8

{"sessionId":"<borrower-session>"}
```

Observed status: `200`. JSON fields:

```json
{
  "institutionId": "<institution-code>",
  "name": "<display-name>",
  "barcode": "<library-card-barcode>",
  "brwIrn": "<local-borrower-id>",
  "brwdIrn": "<consortium-borrower-id>",
  "brwSessionId": "<validated-borrower-session>"
}
```

No Authorization or Cookie header was present on this captured validation request. The opaque session in the body is itself the credential. Use the returned session consistently; validation does not establish expiry, refresh, invalid-session, or cross-library behavior.

The original app's notification request uses `Authorization: Bearer <access-token>` and the **consortium ID** (`brwdIrn`) in its URL. Other contracts use the **local ID** (`brwIrn`/`patronIrn`), so these identifiers must remain separate.

## Web account cookies and routing

The compiled app injects an OPAC cookie session instead of sending the bearer header to the account pages. Decoded string/call-site evidence and authenticated Canning GETs establish these cookie names:

| Cookie | Purpose / observed value |
| --- | --- |
| `SPYDUS_BRWSESSIONID_443` | Borrower session; private, host-scoped credential |
| `SPYDUS_DISPLAYCONTEXT_443` | `MOB`, mobile presentation |
| `consentSettings_443` | `ALL`, app-provided presentation setting |
| `SPYDUS_MYACCOUNT_443` | Empty string in compiled dictionary construction |
| `SPYDUS_DYSLEXICFONT` | App accessibility setting; value depends on user settings |

`SPYDUS_MYACCOUNT_443` uses `System.String.Empty` immediately after its decoded name in method `0x060007d6` (name at IL 631, `ldsfld` at IL 645). The cookie's complete server semantics are not established by this assignment alone.

The recovered account entry template is `/cgi-bin/spydus.exe/PGM/{context}/CCOPT/LB/1?&RFB=1`. Authenticated Canning reads succeeded in the `OPAC` context, with an `OPAC` meta-refresh target. An earlier test using the institution context `WQUP` produced a `WQUP` meta-refresh target that returned `400`; the equivalent target using `OPAC` returned the signed-in page. Institution ID and URL context are therefore distinct in this observed deployment. Treat this as a Canning-specific routing observation, not a rewrite rule for every library.

Canning's selected base language uses an **empty** `spydus-lang` header in captured branches, containers, announcements, and catalogue search. Do not substitute the app config default `ENG` blindly. Search returns a JSON string containing an OPAC URL; account, loans and reservations return HTML. See [web-endpoints.md](web-endpoints.md) for navigation, parsing fields and action references.

## Evidence boundary

Native session/network captures were inspected by REA, with original credential-bearing files retained only under `.private/`. Static client declarations and DTO attributes are bound to assembly hashes and metadata tokens in [native-endpoints.json](native-endpoints.json). Authentication code snippets are decoded statically by [decode_app_strings.py](../../scripts/decode_app_strings.py); this helper does not execute the app. Fresh login, token issuance, bearer expiry, session expiry, credential errors, logout, and tenant-specific pinning behavior require further verification.
