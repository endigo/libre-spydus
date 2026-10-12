import 'package:dio/dio.dart';
import 'package:flutter_web_auth_2/flutter_web_auth_2.dart';

import '../api/identity_client.dart';
import '../config/app_config.dart';
import 'jwt_claims.dart';
import 'token_store.dart';

/// Drives the OAuth2 authorization-code flow against `idp.spydus.com` and the
/// token lifecycle, mirroring the app's `AuthenticationService`.
class AuthService {
  AuthService({required this.identity, required this.tokenStore});

  final IdentityClient identity;
  final TokenStore tokenStore;

  /// Builds the authorize URL. Matches `OAuthUriBuilderExtensions`:
  /// client_id, redirect_uri, response_type, scope (+ prompt=login when forced).
  Uri buildAuthorizeUrl({bool forceInteractive = false, String? extraScope}) {
    final scope = extraScope == null ? AppConfig.scope : '${AppConfig.scope} $extraScope';
    final params = <String, String>{
      'client_id': AppConfig.clientId,
      'redirect_uri': AppConfig.redirectUri,
      'response_type': AppConfig.responseType,
      'scope': scope,
      if (forceInteractive) 'prompt': 'login',
    };
    return Uri.parse('${AppConfig.identityBaseUrl}${AppConfig.authorizePath}')
        .replace(queryParameters: params);
  }

  /// Full interactive login: opens the system browser, captures the
  /// `spyduslibrary://auth?code=...` redirect, exchanges the code for tokens.
  Future<AuthTokens> login({bool forceInteractive = false}) async {
    final authorizeUrl = buildAuthorizeUrl(forceInteractive: forceInteractive);
    final result = await FlutterWebAuth2.authenticate(
      url: authorizeUrl.toString(),
      callbackUrlScheme: AppConfig.redirectScheme,
    );
    final code = Uri.parse(result).queryParameters['code'];
    if (code == null || code.isEmpty) {
      throw const AuthException('Authorization did not return a code');
    }
    return exchangeCode(code);
  }

  /// Exchanges an authorization code for tokens (`GET /openid/token`).
  Future<AuthTokens> exchangeCode(String code) async {
    final res = await identity.exchangeCode(code: code, clientId: AppConfig.clientId);
    if (!res.hasTokens) {
      throw const AuthException('Token endpoint did not return tokens');
    }
    final tokens = AuthTokens(accessToken: res.accessToken!, refreshToken: res.refreshToken!);
    await tokenStore.write(tokens);
    return tokens;
  }

  /// Refreshes tokens (`GET /openid/refreshtoken`). Used by [AuthInterceptor].
  Future<AuthTokens?> refresh(AuthTokens current) async {
    try {
      final res = await identity.refresh(
        refreshToken: current.refreshToken,
        clientId: AppConfig.clientId,
      );
      if (!res.hasTokens) return null;
      final tokens = AuthTokens(accessToken: res.accessToken!, refreshToken: res.refreshToken!);
      await tokenStore.write(tokens);
      return tokens;
    } on DioException {
      return null;
    }
  }

  /// Logs out server-side and clears local tokens.
  Future<void> logout() async {
    final tokens = await tokenStore.read();
    if (tokens != null && tokens.isValid) {
      try {
        await identity.logout(refreshToken: tokens.refreshToken, clientId: AppConfig.clientId);
      } on DioException {
        // best-effort; clear locally regardless
      }
    }
    await tokenStore.clear();
  }

  /// Borrower identity from the current access token, if signed in.
  Future<BorrowerClaims?> currentBorrower() async {
    final tokens = await tokenStore.read();
    if (tokens == null || !tokens.isValid) return null;
    return BorrowerClaims.fromJwt(tokens.accessToken);
  }

  Future<bool> get isSignedIn async => (await tokenStore.read())?.isValid ?? false;
}

class AuthException implements Exception {
  final String message;
  const AuthException(this.message);
  @override
  String toString() => 'AuthException: $message';
}
