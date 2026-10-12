// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'notifications.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

NotificationDto _$NotificationDtoFromJson(Map<String, dynamic> json) =>
    NotificationDto(
      borrowerConsortiumRecordIdentifier: (json['brwdIrn'] as num?)?.toInt(),
      borrowerLocalAccountIdentifier: (json['brwIrn'] as num?)?.toInt(),
      createdDate: json['createdDate'] == null
          ? null
          : DateTime.parse(json['createdDate'] as String),
      id: json['ntfId'] as String?,
      instCode: json['instCode'] as String?,
      isRead: json['isRead'] as bool?,
      libraryId: json['libraryId'] as String?,
      message: json['message'] as String?,
      subject: json['subject'] as String?,
      url: json['url'] as String?,
    );

Map<String, dynamic> _$NotificationDtoToJson(NotificationDto instance) =>
    <String, dynamic>{
      'brwdIrn': instance.borrowerConsortiumRecordIdentifier,
      'brwIrn': instance.borrowerLocalAccountIdentifier,
      'createdDate': instance.createdDate?.toIso8601String(),
      'ntfId': instance.id,
      'instCode': instance.instCode,
      'isRead': instance.isRead,
      'libraryId': instance.libraryId,
      'message': instance.message,
      'subject': instance.subject,
      'url': instance.url,
    };

NotificationResponseDto _$NotificationResponseDtoFromJson(
  Map<String, dynamic> json,
) => NotificationResponseDto(
  notifications: (json['notifications'] as List<dynamic>?)
      ?.map((e) => NotificationDto.fromJson(e as Map<String, dynamic>))
      .toList(),
  totalCount: (json['totalCount'] as num?)?.toInt(),
);

Map<String, dynamic> _$NotificationResponseDtoToJson(
  NotificationResponseDto instance,
) => <String, dynamic>{
  'notifications': instance.notifications,
  'totalCount': instance.totalCount,
};

RegisterDeviceTokenRequest _$RegisterDeviceTokenRequestFromJson(
  Map<String, dynamic> json,
) => RegisterDeviceTokenRequest(
  deviceToken: json['deviceToken'] as String?,
  platform: json['platform'] as String?,
  tags: (json['tags'] as List<dynamic>?)?.map((e) => e as String).toList(),
);

Map<String, dynamic> _$RegisterDeviceTokenRequestToJson(
  RegisterDeviceTokenRequest instance,
) => <String, dynamic>{
  'deviceToken': instance.deviceToken,
  'platform': instance.platform,
  'tags': instance.tags,
};

UpdateDeviceTokenRequest _$UpdateDeviceTokenRequestFromJson(
  Map<String, dynamic> json,
) => UpdateDeviceTokenRequest(
  newDeviceToken: json['newDeviceToken'] as String?,
  oldDeviceToken: json['oldDeviceToken'] as String?,
  platform: json['platform'] as String?,
);

Map<String, dynamic> _$UpdateDeviceTokenRequestToJson(
  UpdateDeviceTokenRequest instance,
) => <String, dynamic>{
  'newDeviceToken': instance.newDeviceToken,
  'oldDeviceToken': instance.oldDeviceToken,
  'platform': instance.platform,
};
