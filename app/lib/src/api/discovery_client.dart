import 'package:dio/dio.dart';
import 'package:retrofit/retrofit.dart';

import '../models/models.dart';

part 'discovery_client.g.dart';

/// Host #1 — global library directory (`https://librarylist.spydus.com/api/1.0`).
/// Mirrors `Spydus.Domain.Interface.ILibrariesClient`. Public, no auth.
@RestApi()
abstract class DiscoveryClient {
  factory DiscoveryClient(Dio dio, {String? baseUrl}) = _DiscoveryClient;

  /// `ILibrariesClient.GetLibraries`
  @GET('/mobile/libraries')
  Future<List<LibraryDto>> getLibraries({@Query('groupCode') String? groupCode});

  /// `ILibrariesClient.GetLanguages`
  @GET('/mobile/languages')
  Future<List<AppLanguageDto>> getLanguages({@Query('groupCode') String? groupCode});
}
