// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'self_issue.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

SelfIssueAlertDto _$SelfIssueAlertDtoFromJson(Map<String, dynamic> json) =>
    SelfIssueAlertDto(
      code: json['code'] as String?,
      critical: json['critical'] as bool?,
      irn: (json['irn'] as num?)?.toInt(),
      ref: json['ref'] as String?,
      text: json['text'] as String?,
    );

Map<String, dynamic> _$SelfIssueAlertDtoToJson(SelfIssueAlertDto instance) =>
    <String, dynamic>{
      'code': instance.code,
      'critical': instance.critical,
      'irn': instance.irn,
      'ref': instance.ref,
      'text': instance.text,
    };

SelfIssueResponseDto _$SelfIssueResponseDtoFromJson(
  Map<String, dynamic> json,
) => SelfIssueResponseDto(
  alerts: (json['alerts'] as List<dynamic>?)
      ?.map((e) => SelfIssueAlertDto.fromJson(e as Map<String, dynamic>))
      .toList(),
  bibAuthor: json['bibAuthor'] as String?,
  bibIssueTitle: json['bibIssueTitle'] as String?,
  bibTitle: json['bibTitle'] as String?,
  coverImage: json['coverImage'] as String?,
  dueDate: json['dueDate'] == null
      ? null
      : DateTime.parse(json['dueDate'] as String),
  isIssued: json['isIssued'] as bool?,
  itemCheckedIn: json['itemCheckedIn'] as bool?,
  itemId: json['itemId'] as String?,
  itemIrn: (json['itemIrn'] as num?)?.toInt(),
  itemRfid: json['itemRfid'] as String?,
  loanIrn: (json['loanIrn'] as num?)?.toInt(),
  patronIrn: (json['patronIrn'] as num?)?.toInt(),
  rentalAmount: (json['rentalAmount'] as num?)?.toDouble(),
);

Map<String, dynamic> _$SelfIssueResponseDtoToJson(
  SelfIssueResponseDto instance,
) => <String, dynamic>{
  'alerts': instance.alerts,
  'bibAuthor': instance.bibAuthor,
  'bibIssueTitle': instance.bibIssueTitle,
  'bibTitle': instance.bibTitle,
  'coverImage': instance.coverImage,
  'dueDate': instance.dueDate?.toIso8601String(),
  'isIssued': instance.isIssued,
  'itemCheckedIn': instance.itemCheckedIn,
  'itemId': instance.itemId,
  'itemIrn': instance.itemIrn,
  'itemRfid': instance.itemRfid,
  'loanIrn': instance.loanIrn,
  'patronIrn': instance.patronIrn,
  'rentalAmount': instance.rentalAmount,
};
