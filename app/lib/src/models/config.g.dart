// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'config.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ServerInfo _$ServerInfoFromJson(Map<String, dynamic> json) => ServerInfo(
  client: json['client'] as String?,
  release: json['release'] as String?,
  license: json['license'] as String?,
  server: json['server'] as String?,
  patch: json['patch'] as String?,
);

Map<String, dynamic> _$ServerInfoToJson(ServerInfo instance) =>
    <String, dynamic>{
      'client': instance.client,
      'release': instance.release,
      'license': instance.license,
      'server': instance.server,
      'patch': instance.patch,
    };

LanguageDto _$LanguageDtoFromJson(Map<String, dynamic> json) => LanguageDto(
  desc: json['desc'] as String?,
  id: json['id'] as String?,
  iso: json['iso'] as String?,
);

Map<String, dynamic> _$LanguageDtoToJson(LanguageDto instance) =>
    <String, dynamic>{
      'desc': instance.desc,
      'id': instance.id,
      'iso': instance.iso,
    };

Network _$NetworkFromJson(Map<String, dynamic> json) => Network(
  actions: (json['actions'] as List<dynamic>?)
      ?.map((e) => e as String)
      .toList(),
  description: json['description'] as String?,
  location: json['location'] as String?,
  name: json['name'] as String?,
);

Map<String, dynamic> _$NetworkToJson(Network instance) => <String, dynamic>{
  'actions': instance.actions,
  'description': instance.description,
  'location': instance.location,
  'name': instance.name,
};

Wifi _$WifiFromJson(Map<String, dynamic> json) => Wifi(
  networks: (json['networks'] as List<dynamic>?)
      ?.map((e) => Network.fromJson(e as Map<String, dynamic>))
      .toList(),
);

Map<String, dynamic> _$WifiToJson(Wifi instance) => <String, dynamic>{
  'networks': instance.networks,
};

DiscoveryDto _$DiscoveryDtoFromJson(Map<String, dynamic> json) => DiscoveryDto(
  searchType: json['searchType'] as String?,
  defaultSort: json['defaultSort'] as String?,
  advancedSearchUrl: json['advancedSearchUrl'] as String?,
);

Map<String, dynamic> _$DiscoveryDtoToJson(DiscoveryDto instance) =>
    <String, dynamic>{
      'searchType': instance.searchType,
      'defaultSort': instance.defaultSort,
      'advancedSearchUrl': instance.advancedSearchUrl,
    };

StyleConfigDto _$StyleConfigDtoFromJson(Map<String, dynamic> json) =>
    StyleConfigDto(
      barcodeType: json['barcodeType'] as String?,
      branchUrl: json['branchUrl'] as String?,
      contentContainerUrl: json['contentContainerUrl'] as String?,
      langs: (json['langs'] as List<dynamic>?)
          ?.map((e) => LanguageDto.fromJson(e as Map<String, dynamic>))
          .toList(),
      logoUrl: json['logoUrl'] as String?,
      notificationHubConnectionString:
          json['notificationHubConnectionString'] as String?,
      notificationHubName: json['notificationHubName'] as String?,
      selfIssueEnabled: json['selfIssueEnabled'] as bool?,
      selfReturnEnabled: json['selfReturnEnabled'] as bool?,
      selfIssueWiFiOnly: json['selfIssueWiFiOnly'] as bool?,
      selfReturnWiFiOnly: json['selfReturnWiFiOnly'] as bool?,
      enableJoinOnline: json['enableJoinOnline'] as bool?,
      spydusViewOverride: json['spydusViewOverride'] as String?,
      style: (json['style'] as Map<String, dynamic>?)?.map(
        (k, e) => MapEntry(k, e as String),
      ),
      wifi: json['wifi'] == null
          ? null
          : Wifi.fromJson(json['wifi'] as Map<String, dynamic>),
      discovery: json['discovery'] == null
          ? null
          : DiscoveryDto.fromJson(json['discovery'] as Map<String, dynamic>),
      joinOnlineIcon: json['joinOnlineIcon'] as String?,
      distanceUnit: json['distanceUnit'] as String?,
      libraryAppTimeZone: json['libraryAppTimeZone'] as String?,
      showNearbyBranches: json['showNearbyBranches'] as bool?,
    );

Map<String, dynamic> _$StyleConfigDtoToJson(
  StyleConfigDto instance,
) => <String, dynamic>{
  'barcodeType': instance.barcodeType,
  'branchUrl': instance.branchUrl,
  'contentContainerUrl': instance.contentContainerUrl,
  'langs': instance.langs,
  'logoUrl': instance.logoUrl,
  'notificationHubConnectionString': instance.notificationHubConnectionString,
  'notificationHubName': instance.notificationHubName,
  'selfIssueEnabled': instance.selfIssueEnabled,
  'selfReturnEnabled': instance.selfReturnEnabled,
  'selfIssueWiFiOnly': instance.selfIssueWiFiOnly,
  'selfReturnWiFiOnly': instance.selfReturnWiFiOnly,
  'enableJoinOnline': instance.enableJoinOnline,
  'spydusViewOverride': instance.spydusViewOverride,
  'style': instance.style,
  'wifi': instance.wifi,
  'discovery': instance.discovery,
  'joinOnlineIcon': instance.joinOnlineIcon,
  'distanceUnit': instance.distanceUnit,
  'libraryAppTimeZone': instance.libraryAppTimeZone,
  'showNearbyBranches': instance.showNearbyBranches,
};
