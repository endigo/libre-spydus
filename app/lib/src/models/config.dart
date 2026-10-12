import 'package:json_annotation/json_annotation.dart';

part 'config.g.dart';

/// Mirrors `Spydus.Domain.DTO.ServerInfo` (`/api/open/1.0/general/version`).
@JsonSerializable()
class ServerInfo {
  final String? client;
  final String? release;
  final String? license;
  final String? server;
  final String? patch;

  ServerInfo({this.client, this.release, this.license, this.server, this.patch});

  factory ServerInfo.fromJson(Map<String, dynamic> json) => _$ServerInfoFromJson(json);
  Map<String, dynamic> toJson() => _$ServerInfoToJson(this);
}

/// Mirrors `Spydus.Domain.DTO.LanguageDto` (inside StyleConfig `langs`).
@JsonSerializable()
class LanguageDto {
  final String? desc;
  final String? id;
  final String? iso;

  LanguageDto({this.desc, this.id, this.iso});

  String? get languageName => iso?.split('-').first;

  factory LanguageDto.fromJson(Map<String, dynamic> json) => _$LanguageDtoFromJson(json);
  Map<String, dynamic> toJson() => _$LanguageDtoToJson(this);
}

/// Mirrors `Spydus.Domain.DTO.Network`.
@JsonSerializable()
class Network {
  final List<String>? actions;
  final String? description;
  final String? location;
  final String? name;

  Network({this.actions, this.description, this.location, this.name});

  factory Network.fromJson(Map<String, dynamic> json) => _$NetworkFromJson(json);
  Map<String, dynamic> toJson() => _$NetworkToJson(this);
}

/// Mirrors `Spydus.Domain.DTO.Wifi`.
@JsonSerializable()
class Wifi {
  final List<Network>? networks;

  Wifi({this.networks});

  factory Wifi.fromJson(Map<String, dynamic> json) => _$WifiFromJson(json);
  Map<String, dynamic> toJson() => _$WifiToJson(this);
}

/// Mirrors `Spydus.Domain.DTO.DiscoveryDto`.
@JsonSerializable()
class DiscoveryDto {
  final String? searchType;
  final String? defaultSort;
  final String? advancedSearchUrl;

  DiscoveryDto({this.searchType, this.defaultSort, this.advancedSearchUrl});

  factory DiscoveryDto.fromJson(Map<String, dynamic> json) => _$DiscoveryDtoFromJson(json);
  Map<String, dynamic> toJson() => _$DiscoveryDtoToJson(this);
}

/// Mirrors `Spydus.Domain.DTO.StyleConfigDto`
/// (`/api/mobile/1.0/general/config` — returned as a JSON string).
/// `selfIssueWiFiOnly` / `selfReturnWiFiOnly` were seen on the wire (Canning)
/// though absent from the compiled DTO; included for fidelity.
@JsonSerializable()
class StyleConfigDto {
  final String? barcodeType;
  final String? branchUrl;
  final String? contentContainerUrl;
  final List<LanguageDto>? langs;
  final String? logoUrl;
  final String? notificationHubConnectionString;
  final String? notificationHubName;
  final bool? selfIssueEnabled;
  final bool? selfReturnEnabled;
  final bool? selfIssueWiFiOnly;
  final bool? selfReturnWiFiOnly;
  final bool? enableJoinOnline;
  final String? spydusViewOverride;
  final Map<String, String>? style;
  final Wifi? wifi;
  final DiscoveryDto? discovery;
  final String? joinOnlineIcon;
  final String? distanceUnit;
  final String? libraryAppTimeZone;
  final bool? showNearbyBranches;

  StyleConfigDto({
    this.barcodeType,
    this.branchUrl,
    this.contentContainerUrl,
    this.langs,
    this.logoUrl,
    this.notificationHubConnectionString,
    this.notificationHubName,
    this.selfIssueEnabled,
    this.selfReturnEnabled,
    this.selfIssueWiFiOnly,
    this.selfReturnWiFiOnly,
    this.enableJoinOnline,
    this.spydusViewOverride,
    this.style,
    this.wifi,
    this.discovery,
    this.joinOnlineIcon,
    this.distanceUnit,
    this.libraryAppTimeZone,
    this.showNearbyBranches,
  });

  factory StyleConfigDto.fromJson(Map<String, dynamic> json) => _$StyleConfigDtoFromJson(json);
  Map<String, dynamic> toJson() => _$StyleConfigDtoToJson(this);
}
