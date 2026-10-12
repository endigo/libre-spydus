import 'package:dio/dio.dart';
import 'package:retrofit/retrofit.dart';

import '../models/models.dart';

part 'identity_client.g.dart';

/// Host #2 — OpenID / OAuth2 identity provider (`https://idp.spydus.com`).
/// Mirrors `IAuthClient` + `ILogOutClient`.
///
/// Note the original method names are counterintuitive: the compiled
/// `GetRefreshToken` performs the authorization-code EXCHANGE (`/openid/token`),
/// and `GetAccessToken` performs the REFRESH (`/openid/refreshtoken`).
/// Names are kept descriptive here.
@RestApi()
abstract class IdentityClient {
  factory IdentityClient(Dio dio, {String? baseUrl}) = _IdentityClient;

  /// Exchange an authorization code for tokens. `IAuthClient.GetRefreshToken`.
  /// Response uses snake_case keys (`access_token`/`refresh_token`).
  @GET('/openid/token')
  Future<RefreshTokenResponse> exchangeCode({
    @Query('code') required String code,
    @Query('grant_type') String grantType = 'authorization_code',
    @Query('client_id') required String clientId,
  });

  /// Refresh an access token. `IAuthClient.GetAccessToken`.
  /// Response uses camelCase keys (`accessToken`/`refreshToken`).
  @GET('/openid/refreshtoken')
  Future<AccessTokenResponse> refresh({
    @Query('refreshToken') required String refreshToken,
    @Query('clientId') required String clientId,
  });

  /// `ILogOutClient.LogOut`
  @POST('/openid/patronlogout')
  Future<void> logout({
    @Query('refreshToken') required String refreshToken,
    @Query('clientId') required String clientId,
  });
}
