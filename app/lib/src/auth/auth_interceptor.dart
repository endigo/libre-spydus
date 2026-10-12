import 'package:dio/dio.dart';

import 'token_store.dart';

/// Injects `Authorization: Bearer <accessToken>` and refreshes once on 401,
/// mirroring the app's bearer `DelegatingHandler`.
///
/// [refresh] returns new tokens (or null if refresh failed / not possible).
class AuthInterceptor extends QueuedInterceptor {
  AuthInterceptor({required this.tokenStore, required this.refresh});

  final TokenStore tokenStore;
  final Future<AuthTokens?> Function(AuthTokens current) refresh;

  @override
  void onRequest(RequestOptions options, RequestInterceptorHandler handler) async {
    if (options.headers['Authorization'] == null) {
      final tokens = await tokenStore.read();
      if (tokens != null && tokens.isValid) {
        options.headers['Authorization'] = 'Bearer ${tokens.accessToken}';
      }
    }
    handler.next(options);
  }

  @override
  void onError(DioException err, ErrorInterceptorHandler handler) async {
    final response = err.response;
    final alreadyRetried = err.requestOptions.extra['__retried__'] == true;
    if (response?.statusCode != 401 || alreadyRetried) {
      return handler.next(err);
    }

    final current = await tokenStore.read();
    if (current == null || !current.isValid) {
      return handler.next(err);
    }

    final refreshed = await refresh(current);
    if (refreshed == null || !refreshed.isValid) {
      return handler.next(err);
    }
    await tokenStore.write(refreshed);

    final req = err.requestOptions;
    req.extra['__retried__'] = true;
    req.headers['Authorization'] = 'Bearer ${refreshed.accessToken}';
    try {
      final dio = Dio(BaseOptions(baseUrl: req.baseUrl));
      final clone = await dio.fetch(req);
      return handler.resolve(clone);
    } on DioException catch (e) {
      return handler.next(e);
    }
  }
}
