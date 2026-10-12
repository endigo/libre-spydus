// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'library.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

LocationDto _$LocationDtoFromJson(Map<String, dynamic> json) => LocationDto(
  country: json['country'] as String?,
  state: json['state'] as String?,
  latitude: (json['latitude'] as num?)?.toDouble(),
  longitude: (json['longitude'] as num?)?.toDouble(),
);

Map<String, dynamic> _$LocationDtoToJson(LocationDto instance) =>
    <String, dynamic>{
      'country': instance.country,
      'state': instance.state,
      'latitude': instance.latitude,
      'longitude': instance.longitude,
    };

LibraryDto _$LibraryDtoFromJson(Map<String, dynamic> json) => LibraryDto(
  name: json['name'] as String?,
  id: json['id'] as String?,
  url: json['url'] as String?,
  logo: json['logo'] as String?,
  activationCode: json['activationCode'] as String?,
  location: json['location'] == null
      ? null
      : LocationDto.fromJson(json['location'] as Map<String, dynamic>),
  pinningHash: (json['pinningHash'] as List<dynamic>?)
      ?.map((e) => e as String)
      .toList(),
  groupCode: json['groupCode'] as String?,
);

Map<String, dynamic> _$LibraryDtoToJson(LibraryDto instance) =>
    <String, dynamic>{
      'name': instance.name,
      'id': instance.id,
      'url': instance.url,
      'logo': instance.logo,
      'activationCode': instance.activationCode,
      'location': instance.location,
      'pinningHash': instance.pinningHash,
      'groupCode': instance.groupCode,
    };

AppLanguageDto _$AppLanguageDtoFromJson(Map<String, dynamic> json) =>
    AppLanguageDto(
      isoCode: json['iso-code'] as String?,
      languagePac: json['language-pac'] as String?,
      name: json['name'] as String?,
    );

Map<String, dynamic> _$AppLanguageDtoToJson(AppLanguageDto instance) =>
    <String, dynamic>{
      'iso-code': instance.isoCode,
      'language-pac': instance.languagePac,
      'name': instance.name,
    };
