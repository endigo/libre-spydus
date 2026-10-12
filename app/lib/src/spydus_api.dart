import 'dart:convert';

import 'api/catalog_client.dart';
import 'api/discovery_client.dart';
import 'api/identity_client.dart';
import 'api/patron_client.dart';
import 'auth/auth_service.dart';
import 'auth/token_store.dart';
import 'config/app_config.dart';
import 'core/dio_factory.dart';
import 'models/models.dart';

/// Top-level client for the Spydus clone (single-library build: City of Canning).
///
/// Wires the three backends, the OAuth flow, and the bearer-authenticated
/// per-library client. Use [create] for the on-device wiring, or construct
/// directly with custom clients/stores for tests.
class SpydusApi {
  SpydusApi({
    required this.discovery,
    required this.catalog,
    required this.patron,
    required this.identity,
    required this.auth,
    required this.tokenStore,
  });

  final DiscoveryClient discovery;
  final CatalogClient catalog;
  final PatronClient patron;
  final IdentityClient identity;
  final AuthService auth;
  final TokenStore tokenStore;

  /// Builds the full stack using secure on-device token storage.
  factory SpydusApi.create({TokenStore? tokenStore, bool logRequests = false}) {
    final store = tokenStore ?? SecureTokenStore();

    // Identity client first; AuthService.refresh is wired into the Dio factory.
    late final AuthService authService;
    final factory = DioFactory(
      tokenStore: store,
      refresh: (current) => authService.refresh(current),
      logRequests: logRequests,
    );

    final identityClient = IdentityClient(factory.identity());
    authService = AuthService(identity: identityClient, tokenStore: store);

    return SpydusApi(
      discovery: DiscoveryClient(factory.discovery()),
      catalog: CatalogClient(factory.library()),
      patron: PatronClient(factory.libraryAuthenticated()),
      identity: identityClient,
      auth: authService,
      tokenStore: store,
    );
  }

  // --- Convenience helpers ---

  /// Directory of all Spydus libraries (host #1).
  Future<List<LibraryDto>> libraries({String? groupCode}) =>
      discovery.getLibraries(groupCode: groupCode);

  /// Server version for the selected library (public).
  Future<ServerInfo> serverVersion() => catalog.getServerVersion();

  /// Library configuration + feature flags. The endpoint returns a JSON string,
  /// which is parsed here into [StyleConfigDto].
  Future<StyleConfigDto> styleConfig({String? wifiName}) async {
    final raw = await catalog.getStyleConfiguration(wifiName: wifiName);
    final decoded = jsonDecode(raw) as Map<String, dynamic>;
    return StyleConfigDto.fromJson(decoded);
  }

  /// Home-screen tiles.
  Future<ContainerResponse> containers({String? wifiName}) =>
      catalog.getContainers(spydusLang: AppConfig.spydusLang, wifiName: wifiName);

  /// Active announcements.
  Future<AnnouncementResponseDto> announcements() =>
      catalog.getAnnouncements(spydusLang: AppConfig.spydusLang);

  /// Branches (single page). Defaults mirror the app's full-list request.
  Future<BranchResponseDto> branches({int setId = 0, int lastIrn = 0, int count = 500}) =>
      catalog.getBranches(
        setId: setId,
        lastIrn: lastIrn,
        count: count,
        spydusLang: AppConfig.spydusLang,
      );

  /// Keyword catalogue search. Returns a JSON string that contains an OPAC URL
  /// (render in a WebView). `barcodeData` search is also available via [catalog].
  Future<String> searchKeywords(String query) =>
      catalog.search(keywords: query, spydusLang: AppConfig.spydusLang);

  /// Patron notifications (requires sign-in; uses consortium id from the JWT).
  Future<NotificationResponseDto> notifications() async {
    final borrower = await auth.currentBorrower();
    final brwdIrn = int.tryParse(borrower?.consortiumRecordId ?? '');
    if (brwdIrn == null) {
      throw StateError('Not signed in, or missing BrwdIrn claim');
    }
    return patron.getNotifications(brwdIrn);
  }
}
