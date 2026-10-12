// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'session.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ValidateSessionRequest _$ValidateSessionRequestFromJson(
  Map<String, dynamic> json,
) => ValidateSessionRequest(borrowerSessionId: json['sessionId'] as String?);

Map<String, dynamic> _$ValidateSessionRequestToJson(
  ValidateSessionRequest instance,
) => <String, dynamic>{'sessionId': instance.borrowerSessionId};

ValidateSessionResponse _$ValidateSessionResponseFromJson(
  Map<String, dynamic> json,
) => ValidateSessionResponse(
  institutionId: json['institutionId'] as String?,
  name: json['name'] as String?,
  barcode: json['barcode'] as String?,
  borrowerLocalAccountIdentifier: json['brwIrn'] as String?,
  borrowerConsortiumRecordIdentifier: json['brwdIrn'] as String?,
  borrowerSessionId: json['brwSessionId'] as String?,
);

Map<String, dynamic> _$ValidateSessionResponseToJson(
  ValidateSessionResponse instance,
) => <String, dynamic>{
  'institutionId': instance.institutionId,
  'name': instance.name,
  'barcode': instance.barcode,
  'brwIrn': instance.borrowerLocalAccountIdentifier,
  'brwdIrn': instance.borrowerConsortiumRecordIdentifier,
  'brwSessionId': instance.borrowerSessionId,
};
