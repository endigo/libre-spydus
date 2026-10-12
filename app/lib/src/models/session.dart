import 'package:json_annotation/json_annotation.dart';

part 'session.g.dart';

/// Mirrors `Spydus.Domain.DTO.ValidateSessionRequest`
/// (body of `POST /api/private/1.0/patrons/validatesessionid`).
@JsonSerializable()
class ValidateSessionRequest {
  @JsonKey(name: 'sessionId')
  final String? borrowerSessionId;

  ValidateSessionRequest({this.borrowerSessionId});

  factory ValidateSessionRequest.fromJson(Map<String, dynamic> json) =>
      _$ValidateSessionRequestFromJson(json);
  Map<String, dynamic> toJson() => _$ValidateSessionRequestToJson(this);
}

/// Mirrors `Spydus.Domain.DTO.ValidateSessionResponse`.
@JsonSerializable()
class ValidateSessionResponse {
  @JsonKey(name: 'institutionId')
  final String? institutionId;
  @JsonKey(name: 'name')
  final String? name;
  @JsonKey(name: 'barcode')
  final String? barcode;
  @JsonKey(name: 'brwIrn')
  final String? borrowerLocalAccountIdentifier;
  @JsonKey(name: 'brwdIrn')
  final String? borrowerConsortiumRecordIdentifier;
  @JsonKey(name: 'brwSessionId')
  final String? borrowerSessionId;

  ValidateSessionResponse({
    this.institutionId,
    this.name,
    this.barcode,
    this.borrowerLocalAccountIdentifier,
    this.borrowerConsortiumRecordIdentifier,
    this.borrowerSessionId,
  });

  factory ValidateSessionResponse.fromJson(Map<String, dynamic> json) =>
      _$ValidateSessionResponseFromJson(json);
  Map<String, dynamic> toJson() => _$ValidateSessionResponseToJson(this);
}
