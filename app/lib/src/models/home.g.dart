// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'home.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ContainerDto _$ContainerDtoFromJson(Map<String, dynamic> json) => ContainerDto(
  title: json['title'] as String?,
  loginRequired: (json['loginRequired'] as num?)?.toInt(),
  icon: json['icon'] as String?,
  url: json['url'] as String?,
  type: json['type'] as String?,
);

Map<String, dynamic> _$ContainerDtoToJson(ContainerDto instance) =>
    <String, dynamic>{
      'title': instance.title,
      'loginRequired': instance.loginRequired,
      'icon': instance.icon,
      'url': instance.url,
      'type': instance.type,
    };

ContainerResponse _$ContainerResponseFromJson(Map<String, dynamic> json) =>
    ContainerResponse(
      containers: (json['containers'] as List<dynamic>?)
          ?.map((e) => ContainerDto.fromJson(e as Map<String, dynamic>))
          .toList(),
    );

Map<String, dynamic> _$ContainerResponseToJson(ContainerResponse instance) =>
    <String, dynamic>{'containers': instance.containers};

AnnouncementDto _$AnnouncementDtoFromJson(Map<String, dynamic> json) =>
    AnnouncementDto(
      title: json['title'] as String?,
      startDate: json['startDate'] == null
          ? null
          : DateTime.parse(json['startDate'] as String),
      endDate: json['endDate'] == null
          ? null
          : DateTime.parse(json['endDate'] as String),
      content: json['content'] as String?,
    );

Map<String, dynamic> _$AnnouncementDtoToJson(AnnouncementDto instance) =>
    <String, dynamic>{
      'title': instance.title,
      'startDate': instance.startDate?.toIso8601String(),
      'endDate': instance.endDate?.toIso8601String(),
      'content': instance.content,
    };

AnnouncementResponseDto _$AnnouncementResponseDtoFromJson(
  Map<String, dynamic> json,
) => AnnouncementResponseDto(
  announcements: (json['announcements'] as List<dynamic>?)
      ?.map((e) => AnnouncementDto.fromJson(e as Map<String, dynamic>))
      .toList(),
);

Map<String, dynamic> _$AnnouncementResponseDtoToJson(
  AnnouncementResponseDto instance,
) => <String, dynamic>{'announcements': instance.announcements};
