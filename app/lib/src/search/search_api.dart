import 'package:dio/dio.dart';

import '../account/account_models.dart';
import '../config/app_config.dart';
import 'record_models.dart';
import 'search_models.dart';

/// Thrown by [MiddlewareApi.login] with a message safe to show the user.
class LoginException implements Exception {
  const LoginException(this.message);
  final String message;
  @override
  String toString() => message;
}

/// Thin client over the local Spydus→JSON middleware.
class MiddlewareApi {
  MiddlewareApi({Dio? dio})
      : _dio = dio ??
            Dio(BaseOptions(
              baseUrl: AppConfig.middlewareBaseUrl,
              // The first search triggers a 5–9 s upstream OPAC render.
              connectTimeout: const Duration(seconds: 10),
              receiveTimeout: const Duration(seconds: 30),
              responseType: ResponseType.json,
            ));

  final Dio _dio;

  Future<SearchResults> search(String query) async {
    final res = await _dio.get<Map<String, dynamic>>(
      '/search',
      queryParameters: {'q': query},
    );
    return SearchResults.fromJson(res.data!);
  }

  /// Fetch a subsequent results page by its absolute OPAC URL
  /// (`SearchResults.nextPageUrl`).
  Future<SearchResults> page(String pageUrl) async {
    final res = await _dio.get<Map<String, dynamic>>(
      '/page',
      queryParameters: {'url': pageUrl},
    );
    return SearchResults.fromJson(res.data!);
  }

  Future<Availability> availability(String availabilityUrl) async {
    final res = await _dio.get<Map<String, dynamic>>(
      '/availability',
      queryParameters: {'url': availabilityUrl},
    );
    return Availability.fromJson(res.data!);
  }

  /// Rich bib record. Pass a format's [availabilityUrl] (BK preferred); the
  /// middleware resolves it to the concrete bib page and parses the details.
  Future<RecordDetail> record(String formatUrl) async {
    final res = await _dio.get<Map<String, dynamic>>(
      '/record',
      queryParameters: {'url': formatUrl},
    );
    return RecordDetail.fromJson(res.data!);
  }

  /// A related-titles panel (similar / author / series / versions).
  Future<RelatedList> related(String panelUrl) async {
    final res = await _dio.get<Map<String, dynamic>>(
      '/related',
      queryParameters: {'url': panelUrl},
    );
    return RelatedList.fromJson(res.data!);
  }

  // --- Authenticated account ---

  /// Logs in with card number + PIN; returns the opaque session token.
  /// The password is sent once to the local middleware (which RSA-encrypts it
  /// for the library, exactly like the web client) and is never stored.
  Future<String> login(String username, String password) async {
    try {
      final res = await _dio.post<Map<String, dynamic>>(
        '/login',
        data: {'username': username, 'password': password},
      );
      final session = res.data?['session'] as String?;
      if (session == null) {
        throw const LoginException('Login failed. Please try again.');
      }
      return session;
    } on DioException catch (e) {
      // Surface the middleware's user-facing reason when it sent one.
      final reason = e.response?.data is Map ? (e.response!.data as Map)['reason'] as String? : null;
      if (reason != null) throw LoginException(reason);
      if (e.type == DioExceptionType.connectionError ||
          e.type == DioExceptionType.connectionTimeout) {
        throw const LoginException('Can\'t reach the sign-in service. Is it running?');
      }
      throw const LoginException('Sign in failed. Please try again.');
    }
  }

  Future<PatronList> loans(String session) async {
    final res = await _dio.get<Map<String, dynamic>>(
      '/account/loans',
      options: Options(headers: {'X-Spydus-Session': session}),
    );
    return PatronList.fromJson(res.data!);
  }

  Future<ReservationsResponse> reservations(String session) async {
    final res = await _dio.get<Map<String, dynamic>>(
      '/account/reservations',
      options: Options(headers: {'X-Spydus-Session': session}),
    );
    return ReservationsResponse.fromJson(res.data!);
  }

  /// Read the reservation request form (pickup location + expiry). No hold is
  /// placed. [reserveUrl] is a search record format's `reserveUrl`.
  Future<ReserveOptions> reserveOptions(String reserveUrl, String session) async {
    final res = await _dio.get<Map<String, dynamic>>(
      '/reserve/options',
      queryParameters: {'url': reserveUrl},
      options: Options(headers: {'X-Spydus-Session': session}),
    );
    return ReserveOptions.fromJson(res.data!);
  }

  /// Place a reservation (mutating). Only called on explicit user confirmation.
  Future<ReserveResult> placeReservation(String reserveUrl, String session, {String? xdays}) async {
    final data = <String, dynamic>{'url': reserveUrl};
    if (xdays != null) data['xdays'] = xdays;
    try {
      final res = await _dio.post<Map<String, dynamic>>(
        '/reserve',
        data: data,
        options: Options(
          headers: {'X-Spydus-Session': session},
          // 409 (couldn't place) carries a message we want to show, not throw.
          validateStatus: (s) => s != null && s < 500,
        ),
      );
      return ReserveResult.fromJson(res.data ?? const {});
    } on DioException {
      return const ReserveResult(ok: false, message: 'Couldn\'t reach the reservation service.');
    }
  }

  /// Cancel a reservation (mutating). [cancelUrl] is the reservation's
  /// `cancelUrl` action. Only called on explicit user confirmation.
  Future<ReserveResult> cancelReservation(String cancelUrl, String session) async {
    try {
      final res = await _dio.post<Map<String, dynamic>>(
        '/reserve/cancel',
        data: {'url': cancelUrl},
        options: Options(
          headers: {'X-Spydus-Session': session},
          validateStatus: (s) => s != null && s < 500,
        ),
      );
      return ReserveResult.fromJson(res.data ?? const {});
    } on DioException {
      return const ReserveResult(ok: false, message: 'Couldn\'t reach the reservation service.');
    }
  }

  /// Read the edit-reservation form (pickup locations + current dates). No
  /// change is made. [editUrl] is the reservation's `editUrl` action.
  Future<EditOptions> editOptions(String editUrl, String session) async {
    final res = await _dio.get<Map<String, dynamic>>(
      '/reserve/edit',
      queryParameters: {'url': editUrl},
      options: Options(headers: {'X-Spydus-Session': session}),
    );
    return EditOptions.fromJson(res.data!);
  }

  /// Edit a reservation (mutating). A null date is left unchanged; pass the
  /// matching `clear*` flag to remove one. Only called on explicit confirmation.
  Future<ReserveResult> editReservation(
    String editUrl,
    String session, {
    String? pickup,
    String? delayUntil,
    String? notRequiredAfter,
    bool clearDelay = false,
    bool clearExpiry = false,
  }) async {
    final data = <String, dynamic>{'url': editUrl};
    if (pickup != null) data['pickup'] = pickup;
    if (delayUntil != null) data['delayUntil'] = delayUntil;
    if (notRequiredAfter != null) data['notRequiredAfter'] = notRequiredAfter;
    if (clearDelay) data['clearDelay'] = true;
    if (clearExpiry) data['clearExpiry'] = true;
    try {
      final res = await _dio.post<Map<String, dynamic>>(
        '/reserve/edit',
        data: data,
        options: Options(
          headers: {'X-Spydus-Session': session},
          validateStatus: (s) => s != null && s < 500,
        ),
      );
      return ReserveResult.fromJson(res.data ?? const {});
    } on DioException {
      return const ReserveResult(ok: false, message: 'Couldn\'t reach the reservation service.');
    }
  }
}
