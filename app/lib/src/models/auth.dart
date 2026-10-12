import 'package:json_annotation/json_annotation.dart';

part 'auth.g.dart';

/// Mirrors `Spydus.Domain.DTO.Auth.RefreshTokenResponse`.
/// Returned by the CODE EXCHANGE endpoint `GET /openid/token` (snake_case keys).
@JsonSerializable()
class RefreshTokenResponse {
  @JsonKey(name: 'access_token')
  final String? accessToken;
  @JsonKey(name: 'refresh_token')
  final String? refreshToken;

  RefreshTokenResponse({this.accessToken, this.refreshToken});

  bool get hasTokens =>
      (accessToken?.isNotEmpty ?? false) && (refreshToken?.isNotEmpty ?? false);

  factory RefreshTokenResponse.fromJson(Map<String, dynamic> json) =>
      _$RefreshTokenResponseFromJson(json);
  Map<String, dynamic> toJson() => _$RefreshTokenResponseToJson(this);
}

/// Mirrors `Spydus.Domain.DTO.Auth.AccessTokenResponse`.
/// Returned by the REFRESH endpoint `GET /openid/refreshtoken` (camelCase keys).
@JsonSerializable()
class AccessTokenResponse {
  @JsonKey(name: 'accessToken')
  final String? accessToken;
  @JsonKey(name: 'refreshToken')
  final String? refreshToken;

  AccessTokenResponse({this.accessToken, this.refreshToken});

  bool get hasTokens =>
      (accessToken?.isNotEmpty ?? false) && (refreshToken?.isNotEmpty ?? false);

  factory AccessTokenResponse.fromJson(Map<String, dynamic> json) =>
      _$AccessTokenResponseFromJson(json);
  Map<String, dynamic> toJson() => _$AccessTokenResponseToJson(this);
}
