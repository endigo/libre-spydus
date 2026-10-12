import 'package:json_annotation/json_annotation.dart';

part 'notifications.g.dart';

/// Mirrors `Spydus.Domain.DTO.NotificationDto`.
@JsonSerializable()
class NotificationDto {
  @JsonKey(name: 'brwdIrn')
  final int? borrowerConsortiumRecordIdentifier;
  @JsonKey(name: 'brwIrn')
  final int? borrowerLocalAccountIdentifier;
  @JsonKey(name: 'createdDate')
  final DateTime? createdDate;
  @JsonKey(name: 'ntfId')
  final String? id;
  @JsonKey(name: 'instCode')
  final String? instCode;
  @JsonKey(name: 'isRead')
  final bool? isRead;
  @JsonKey(name: 'libraryId')
  final String? libraryId;
  @JsonKey(name: 'message')
  final String? message;
  @JsonKey(name: 'subject')
  final String? subject;
  @JsonKey(name: 'url')
  final String? url;

  NotificationDto({
    this.borrowerConsortiumRecordIdentifier,
    this.borrowerLocalAccountIdentifier,
    this.createdDate,
    this.id,
    this.instCode,
    this.isRead,
    this.libraryId,
    this.message,
    this.subject,
    this.url,
  });

  factory NotificationDto.fromJson(Map<String, dynamic> json) => _$NotificationDtoFromJson(json);
  Map<String, dynamic> toJson() => _$NotificationDtoToJson(this);
}

/// Mirrors `Spydus.Domain.DTO.NotificationResponseDto`.
@JsonSerializable()
class NotificationResponseDto {
  @JsonKey(name: 'notifications')
  final List<NotificationDto>? notifications;
  @JsonKey(name: 'totalCount')
  final int? totalCount;

  NotificationResponseDto({this.notifications, this.totalCount});

  factory NotificationResponseDto.fromJson(Map<String, dynamic> json) =>
      _$NotificationResponseDtoFromJson(json);
  Map<String, dynamic> toJson() => _$NotificationResponseDtoToJson(this);
}

/// Mirrors `Spydus.Domain.DTO.RegisterDeviceTokenRequest`.
@JsonSerializable()
class RegisterDeviceTokenRequest {
  @JsonKey(name: 'deviceToken')
  final String? deviceToken;
  @JsonKey(name: 'platform')
  final String? platform;
  @JsonKey(name: 'tags')
  final List<String>? tags;

  RegisterDeviceTokenRequest({this.deviceToken, this.platform, this.tags});

  factory RegisterDeviceTokenRequest.fromJson(Map<String, dynamic> json) =>
      _$RegisterDeviceTokenRequestFromJson(json);
  Map<String, dynamic> toJson() => _$RegisterDeviceTokenRequestToJson(this);
}

/// Mirrors `Spydus.Domain.DTO.UpdateDeviceTokenRequest`.
@JsonSerializable()
class UpdateDeviceTokenRequest {
  @JsonKey(name: 'newDeviceToken')
  final String? newDeviceToken;
  @JsonKey(name: 'oldDeviceToken')
  final String? oldDeviceToken;
  @JsonKey(name: 'platform')
  final String? platform;

  UpdateDeviceTokenRequest({this.newDeviceToken, this.oldDeviceToken, this.platform});

  factory UpdateDeviceTokenRequest.fromJson(Map<String, dynamic> json) =>
      _$UpdateDeviceTokenRequestFromJson(json);
  Map<String, dynamic> toJson() => _$UpdateDeviceTokenRequestToJson(this);
}
