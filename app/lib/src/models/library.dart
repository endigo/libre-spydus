import 'package:json_annotation/json_annotation.dart';

part 'library.g.dart';

/// Mirrors `Spydus.Domain.DTO.LocationDto`.
@JsonSerializable()
class LocationDto {
  final String? country;
  final String? state;
  final double? latitude;
  final double? longitude;

  LocationDto({this.country, this.state, this.latitude, this.longitude});

  factory LocationDto.fromJson(Map<String, dynamic> json) => _$LocationDtoFromJson(json);
  Map<String, dynamic> toJson() => _$LocationDtoToJson(this);
}

/// Mirrors `Spydus.Domain.DTO.LibraryDto` (host #1 `/mobile/libraries`).
/// Wire keys are camelCase; `groupCode` is returned by the directory but is
/// not part of the compiled DTO.
@JsonSerializable()
class LibraryDto {
  final String? name;
  final String? id;
  final String? url;
  final String? logo;
  final String? activationCode;
  final LocationDto? location;
  final List<String>? pinningHash;
  final String? groupCode;

  LibraryDto({
    this.name,
    this.id,
    this.url,
    this.logo,
    this.activationCode,
    this.location,
    this.pinningHash,
    this.groupCode,
  });

  factory LibraryDto.fromJson(Map<String, dynamic> json) => _$LibraryDtoFromJson(json);
  Map<String, dynamic> toJson() => _$LibraryDtoToJson(this);
}

/// Mirrors `Spydus.Domain.DTO.AppLanguageDto` (host #1 `/mobile/languages`).
@JsonSerializable()
class AppLanguageDto {
  @JsonKey(name: 'iso-code')
  final String? isoCode;
  @JsonKey(name: 'language-pac')
  final String? languagePac;
  final String? name;

  AppLanguageDto({this.isoCode, this.languagePac, this.name});

  /// `LanguageName` in the app: first segment of the ISO code.
  String? get languageName => isoCode?.split('-').first;

  factory AppLanguageDto.fromJson(Map<String, dynamic> json) => _$AppLanguageDtoFromJson(json);
  Map<String, dynamic> toJson() => _$AppLanguageDtoToJson(this);
}
