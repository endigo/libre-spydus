import 'package:dio/dio.dart' hide Headers;
import 'package:retrofit/retrofit.dart';

import '../models/models.dart';

part 'catalog_client.g.dart';

/// Host #3 — the selected library's Spydus server, public / session endpoints.
/// Mirrors `IStyleClient` (minus wallet), `ILogoClient`, `IBookClient`,
/// `IValidateSessionClient`. No bearer token required.
@RestApi()
abstract class CatalogClient {
  factory CatalogClient(Dio dio, {String? baseUrl}) = _CatalogClient;

  /// `IStyleClient.GetServerVersion`
  @GET('/api/open/1.0/general/version')
  Future<ServerInfo> getServerVersion();

  /// `IStyleClient.GetStyleConfiguration` — body is a JSON string of StyleConfigDto.
  @GET('/api/mobile/1.0/general/config')
  Future<String> getStyleConfiguration({@Query('wifiName') String? wifiName});

  /// `ILogoClient.GetLogo`
  @GET('/api/mobile/1.0/general/logo')
  Future<String> getLogo();

  /// `IStyleClient.GetContainers`
  @GET('/api/mobile/1.0/containers')
  Future<ContainerResponse> getContainers({
    @Header('spydus-lang') String? spydusLang,
    @Query('wifiName') String? wifiName,
  });

  /// `IStyleClient.GetAnnouncements`
  @GET('/api/mobile/1.0/announcements')
  Future<AnnouncementResponseDto> getAnnouncements({
    @Header('spydus-lang') String? spydusLang,
  });

  /// `IStyleClient.GetBranches`
  @GET('/api/mobile/1.0/mobile/branches')
  Future<BranchResponseDto> getBranches({
    @Query('setId') required int setId,
    @Query('lastIrn') required int lastIrn,
    @Query('count') required int count,
    @Header('spydus-lang') String? spydusLang,
  });

  /// `IBookClient.GetSearchBook` — returns a JSON string containing an OPAC URL.
  /// `searchMode` is `keywords` or `barcodeData` (DiscoverSearchModeEnum).
  @GET('/api/mobile/1.0/discover/search')
  Future<String> search({
    @Query('keywords') String? keywords,
    @Query('barcodeData') String? barcodeData,
    @Header('spydus-lang') String? spydusLang,
  });

  /// `IValidateSessionClient.ValidateSessionId` — the session id in the body is
  /// itself the credential; no Authorization/Cookie header is sent.
  @POST('/api/private/1.0/patrons/validatesessionid')
  @Headers({'Content-Type': 'application/json; charset=UTF-8'})
  Future<ValidateSessionResponse> validateSession(@Body() ValidateSessionRequest request);
}
