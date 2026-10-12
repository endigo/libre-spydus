import 'package:json_annotation/json_annotation.dart';

part 'branches.g.dart';

/// Mirrors `Spydus.Domain.DTO.BranchAddressDto`.
@JsonSerializable()
class BranchAddressDto {
  final String? numberAndStreet;
  final String? streetLine2;
  final String? streetLine3;
  final String? suburb;
  final String? postcode;
  final String? state;
  final String? country;

  BranchAddressDto({
    this.numberAndStreet,
    this.streetLine2,
    this.streetLine3,
    this.suburb,
    this.postcode,
    this.state,
    this.country,
  });

  String get fullAddress => [
        numberAndStreet,
        streetLine2,
        streetLine3,
        suburb,
        postcode,
        state,
        country,
      ].where((e) => e != null && e.trim().isNotEmpty).join(', ');

  factory BranchAddressDto.fromJson(Map<String, dynamic> json) => _$BranchAddressDtoFromJson(json);
  Map<String, dynamic> toJson() => _$BranchAddressDtoToJson(this);
}

/// Mirrors `Spydus.Domain.DTO.OpeningHourDto`.
@JsonSerializable()
class OpeningHourDto {
  final int? dow;
  final String? dayName;
  final String? openTime;
  final String? closeTime;
  final bool? isOpen;

  OpeningHourDto({this.dow, this.dayName, this.openTime, this.closeTime, this.isOpen});

  factory OpeningHourDto.fromJson(Map<String, dynamic> json) => _$OpeningHourDtoFromJson(json);
  Map<String, dynamic> toJson() => _$OpeningHourDtoToJson(this);
}

/// Mirrors `Spydus.Domain.DTO.HolidayDto`.
@JsonSerializable()
class HolidayDto {
  final String? startDate;
  final String? endDate;
  final String? name;

  HolidayDto({this.startDate, this.endDate, this.name});

  factory HolidayDto.fromJson(Map<String, dynamic> json) => _$HolidayDtoFromJson(json);
  Map<String, dynamic> toJson() => _$HolidayDtoToJson(this);
}

/// Mirrors `Spydus.Domain.DTO.BranchDto`.
@JsonSerializable()
class BranchDto {
  final int? irn;
  final String? id;
  final String? name;
  final double? latitude;
  final double? longitude;
  final String? url;
  final String? opacUrl;
  final String? imageUrl;
  final String? phone;
  final String? email;
  final BranchAddressDto? address;
  final List<OpeningHourDto>? openingHours;
  final List<HolidayDto>? holidays;

  BranchDto({
    this.irn,
    this.id,
    this.name,
    this.latitude,
    this.longitude,
    this.url,
    this.opacUrl,
    this.imageUrl,
    this.phone,
    this.email,
    this.address,
    this.openingHours,
    this.holidays,
  });

  factory BranchDto.fromJson(Map<String, dynamic> json) => _$BranchDtoFromJson(json);
  Map<String, dynamic> toJson() => _$BranchDtoToJson(this);
}

/// Mirrors `Spydus.Domain.DTO.SetInfo` (paging envelope).
@JsonSerializable()
class SetInfo {
  final int? setId;
  final int? lastIrn;
  final int? setTotal;
  final int? setCount;
  final bool? hasMoreRecords;

  SetInfo({this.setId, this.lastIrn, this.setTotal, this.setCount, this.hasMoreRecords});

  factory SetInfo.fromJson(Map<String, dynamic> json) => _$SetInfoFromJson(json);
  Map<String, dynamic> toJson() => _$SetInfoToJson(this);
}

/// Mirrors `Spydus.Domain.DTO.BranchResponseDto`
/// (`/api/mobile/1.0/mobile/branches`).
@JsonSerializable()
class BranchResponseDto {
  final SetInfo? setInfo;
  final List<BranchDto>? branches;

  BranchResponseDto({this.setInfo, this.branches});

  factory BranchResponseDto.fromJson(Map<String, dynamic> json) => _$BranchResponseDtoFromJson(json);
  Map<String, dynamic> toJson() => _$BranchResponseDtoToJson(this);
}
