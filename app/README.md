# Spydus Clone (Flutter)

A Flutter reimplementation of the Civica **Spydus Library** mobile app, targeting a
single library: **City of Canning** (`https://canning.spydus.com`).

The networking + auth layer is a faithful port of the original .NET MAUI app's
**Refit** API contract (recovered by reverse-engineering the shipped APK — see
`../docs/` and `../docs/api/`). Each Dart client maps 1:1 to a `Spydus.Domain.Interface.*Client`.

## Architecture

| Layer | Files | Mirrors |
|-------|-------|---------|
| Config | `lib/src/config/app_config.dart` | `assets/app.config`, `AuthConstants` |
| Models (DTOs) | `lib/src/models/*.dart` | `Spydus.Domain.DTO.*` |
| API clients (Retrofit) | `lib/src/api/*.dart` | `Spydus.Domain.Interface.I*Client` |
| OAuth + tokens | `lib/src/auth/*.dart` | `AuthenticationService`, bearer `DelegatingHandler` |
| Dio wiring | `lib/src/core/dio_factory.dart` | `DynamicApiClientFactory` |
| Facade | `lib/src/spydus_api.dart` | app service layer |

### Three backends
- **Discovery** `https://librarylist.spydus.com/api/1.0` — `DiscoveryClient`
- **Identity** `https://idp.spydus.com` — `IdentityClient` (OAuth2 auth-code)
- **Library** `https://canning.spydus.com` — `CatalogClient` (public/session) + `PatronClient` (bearer)

## Run

```bash
flutter pub get
dart run build_runner build          # generate *.g.dart (models + clients)
flutter test                         # 17 tests (models, clients, auth)
flutter run                          # smoke-test screen hits the live API layer
```

> `main.dart` is a diagnostic smoke-test screen, not the final UI. It exercises the
> live public endpoints to validate the data layer.

## Known constraints (from the RE investigation)
- **AWS WAF**: per-library endpoints (`canning.spydus.com`) are fronted by AWS WAF.
  A plain HTTP client gets a "Human Verification" challenge. A real client likely needs
  a hidden WebView to obtain the `aws-waf-token` cookie. Not yet implemented.
- **Certificate pinning**: SPKI pins are in `AppConfig.pinningSpkiSha256`; enforcement
  interceptor is TODO.
- **Search & My Account** return OPAC HTML / a results URL, not pure JSON — a WebView +
  HTML parser is needed for those screens.

## Next steps
1. WAF token acquisition (hidden WebView) + attach cookie to the library Dio.
2. OPAC session bridge (login WebView → `validatesessionid`) for loans/reservations.
3. Real UI: home (containers), branches, search results, notifications, account card.
