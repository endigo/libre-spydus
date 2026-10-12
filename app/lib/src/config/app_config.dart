/// Static configuration for the Spydus clone.
///
/// Values recovered from the original app's `assets/app.config`,
/// `Spydus.Helpers.AuthConstants`, and live traffic to the signed-in library.
/// This build targets a single library: **City of Canning**.
class AppConfig {
  const AppConfig._();

  // --- Host #1: global library directory (public) ---
  static const String discoveryBaseUrl = 'https://librarylist.spydus.com/api/1.0';

  // --- Host #2: OpenID / OAuth2 identity provider ---
  static const String identityBaseUrl = 'https://idp.spydus.com';

  // --- Host #3: the selected library's Spydus server ---
  // Discovered at runtime in the real app (LibraryDto.url); hardcoded here.
  static const String libraryBaseUrl = 'https://canning.spydus.com';
  static const String libraryName = 'City of Canning';

  // --- Local middleware (Bun + Hono) that scrapes OPAC HTML → JSON ---
  // The Android emulator reaches the host machine's loopback via 10.0.2.2.
  // Override at build time with --dart-define=MIDDLEWARE_URL=...
  static const String middlewareBaseUrl = String.fromEnvironment(
    'MIDDLEWARE_URL',
    defaultValue: 'http://10.0.2.2:8787',
  );

  // --- OAuth2 (authorization-code flow) ---
  static const String clientId = 'spydusmobile';
  static const String redirectUri = 'spyduslibrary://auth';
  static const String redirectScheme = 'spyduslibrary';
  static const String authorizePath = '/openid/authorize';
  static const String responseType = 'code';

  /// Base scope. The original app also appends dynamic `libid_<..>` and
  /// `lang_<..>` components for the selected library/language.
  static const String scope = 'openid profile mobile';

  /// Language header sent to per-library endpoints.
  /// Canning's selected base language sends an EMPTY `spydus-lang` header
  /// (observed in live traffic); app.config default is `ENG`.
  static const String spydusLang = '';
  static const String defaultLanguage = 'ENG';

  // --- Certificate pinning (SPKI SHA-256), default pins from app.config. ---
  // Most libraries return null pinningHash and fall back to these.
  static const List<String> pinningSpkiSha256 = <String>[
    '2BWWoN1scd+yGvhqGTiPnKZsxSHfv6FIMFKs2MLmA3A=', // primary
    'PmYXBbYmXsWCxmSRUgnhfYmJ09ETroEOgIITe4AFntI=', // backup
  ];

  // --- Classic OPAC web views (WebView, not JSON APIs). ---
  // {0} = OPAC program/context (observed working value: "OPAC").
  static const String myAccountPath = '/cgi-bin/spydus.exe/PGM/{0}/CCOPT/LB/1?&RFB=1';
  static const String joinOnlinePath = '/cgi-bin/spydus.exe/MSGTRN/{0}/JOINSEL';
  static const String opacPath = '/OPAC/';
  static const String opacContext = 'OPAC';

  static const Duration apiTimeout = Duration(seconds: 30);
}
