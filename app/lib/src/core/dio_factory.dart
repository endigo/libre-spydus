import 'package:dio/dio.dart';

import '../config/app_config.dart';
import '../auth/auth_interceptor.dart';
import '../auth/token_store.dart';

/// Builds the Dio instances for each backend, mirroring the app's
/// `DynamicApiClientFactory` (per-host base address, 30s timeout) and
/// `DynamicAuthenticatedApiClientFactory` (bearer handler).
class DioFactory {
  DioFactory({
    required this.tokenStore,
    required this.refresh,
    this.logRequests = false,
  });

  final TokenStore tokenStore;
  final Future<AuthTokens?> Function(AuthTokens current) refresh;
  final bool logRequests;

  BaseOptions _base(String url) => BaseOptions(
        baseUrl: url,
        connectTimeout: AppConfig.apiTimeout,
        receiveTimeout: AppConfig.apiTimeout,
        sendTimeout: AppConfig.apiTimeout,
        // A browser-like UA; some Spydus tenants front the API with a WAF.
        headers: <String, dynamic>{'User-Agent': 'SpydusLibrary/2.0.6 (Flutter)'},
        // Treat 4xx as responses so interceptors/services can inspect them.
        validateStatus: (code) => code != null && code < 500,
      );

  Dio _withLogging(Dio dio) {
    if (logRequests) {
      dio.interceptors.add(LogInterceptor(requestBody: true, responseBody: false));
    }
    return dio;
  }

  /// Host #1 — library directory (public).
  Dio discovery() => _withLogging(Dio(_base(AppConfig.discoveryBaseUrl)));

  /// Host #2 — identity provider (public token endpoints).
  Dio identity() => _withLogging(Dio(_base(AppConfig.identityBaseUrl)));

  /// Host #3 — selected library, public / session endpoints.
  Dio library() => _withLogging(Dio(_base(AppConfig.libraryBaseUrl)));

  /// Host #3 — selected library, bearer-authenticated endpoints.
  Dio libraryAuthenticated() {
    final dio = Dio(_base(AppConfig.libraryBaseUrl));
    dio.interceptors.add(AuthInterceptor(tokenStore: tokenStore, refresh: refresh));
    return _withLogging(dio);
  }
}
