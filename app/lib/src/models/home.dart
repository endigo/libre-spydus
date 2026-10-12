import 'package:json_annotation/json_annotation.dart';

part 'home.g.dart';

/// Mirrors `Spydus.Domain.DTO.ContainerDto` (home-screen tiles).
/// `loginRequired`: 0 = always, 1 = only when logged in, 2 = only when not.
@JsonSerializable()
class ContainerDto {
  final String? title;
  final int? loginRequired;
  final String? icon;
  final String? url;
  final String? type;

  ContainerDto({this.title, this.loginRequired, this.icon, this.url, this.type});

  bool get onlyWhenLoggedIn => loginRequired == 1;
  bool get onlyWhenNotLoggedIn => loginRequired == 2;

  factory ContainerDto.fromJson(Map<String, dynamic> json) => _$ContainerDtoFromJson(json);
  Map<String, dynamic> toJson() => _$ContainerDtoToJson(this);
}

/// Mirrors `Spydus.Domain.DTO.ContainerResponse` (`/api/mobile/1.0/containers`).
@JsonSerializable()
class ContainerResponse {
  final List<ContainerDto>? containers;

  ContainerResponse({this.containers});

  factory ContainerResponse.fromJson(Map<String, dynamic> json) => _$ContainerResponseFromJson(json);
  Map<String, dynamic> toJson() => _$ContainerResponseToJson(this);
}

/// Mirrors `Spydus.Domain.DTO.AnnouncementDto`.
@JsonSerializable()
class AnnouncementDto {
  final String? title;
  final DateTime? startDate;
  final DateTime? endDate;
  final String? content;

  AnnouncementDto({this.title, this.startDate, this.endDate, this.content});

  factory AnnouncementDto.fromJson(Map<String, dynamic> json) => _$AnnouncementDtoFromJson(json);
  Map<String, dynamic> toJson() => _$AnnouncementDtoToJson(this);
}

/// Mirrors `Spydus.Domain.DTO.AnnouncementResponseDto` (`/api/mobile/1.0/announcements`).
@JsonSerializable()
class AnnouncementResponseDto {
  final List<AnnouncementDto>? announcements;

  AnnouncementResponseDto({this.announcements});

  factory AnnouncementResponseDto.fromJson(Map<String, dynamic> json) =>
      _$AnnouncementResponseDtoFromJson(json);
  Map<String, dynamic> toJson() => _$AnnouncementResponseDtoToJson(this);
}
