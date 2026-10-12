// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'branches.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

BranchAddressDto _$BranchAddressDtoFromJson(Map<String, dynamic> json) =>
    BranchAddressDto(
      numberAndStreet: json['numberAndStreet'] as String?,
      streetLine2: json['streetLine2'] as String?,
      streetLine3: json['streetLine3'] as String?,
      suburb: json['suburb'] as String?,
      postcode: json['postcode'] as String?,
      state: json['state'] as String?,
      country: json['country'] as String?,
    );

Map<String, dynamic> _$BranchAddressDtoToJson(BranchAddressDto instance) =>
    <String, dynamic>{
      'numberAndStreet': instance.numberAndStreet,
      'streetLine2': instance.streetLine2,
      'streetLine3': instance.streetLine3,
      'suburb': instance.suburb,
      'postcode': instance.postcode,
      'state': instance.state,
      'country': instance.country,
    };

OpeningHourDto _$OpeningHourDtoFromJson(Map<String, dynamic> json) =>
    OpeningHourDto(
      dow: (json['dow'] as num?)?.toInt(),
      dayName: json['dayName'] as String?,
      openTime: json['openTime'] as String?,
      closeTime: json['closeTime'] as String?,
      isOpen: json['isOpen'] as bool?,
    );

Map<String, dynamic> _$OpeningHourDtoToJson(OpeningHourDto instance) =>
    <String, dynamic>{
      'dow': instance.dow,
      'dayName': instance.dayName,
      'openTime': instance.openTime,
      'closeTime': instance.closeTime,
      'isOpen': instance.isOpen,
    };

HolidayDto _$HolidayDtoFromJson(Map<String, dynamic> json) => HolidayDto(
  startDate: json['startDate'] as String?,
  endDate: json['endDate'] as String?,
  name: json['name'] as String?,
);

Map<String, dynamic> _$HolidayDtoToJson(HolidayDto instance) =>
    <String, dynamic>{
      'startDate': instance.startDate,
      'endDate': instance.endDate,
      'name': instance.name,
    };

BranchDto _$BranchDtoFromJson(Map<String, dynamic> json) => BranchDto(
  irn: (json['irn'] as num?)?.toInt(),
  id: json['id'] as String?,
  name: json['name'] as String?,
  latitude: (json['latitude'] as num?)?.toDouble(),
  longitude: (json['longitude'] as num?)?.toDouble(),
  url: json['url'] as String?,
  opacUrl: json['opacUrl'] as String?,
  imageUrl: json['imageUrl'] as String?,
  phone: json['phone'] as String?,
  email: json['email'] as String?,
  address: json['address'] == null
      ? null
      : BranchAddressDto.fromJson(json['address'] as Map<String, dynamic>),
  openingHours: (json['openingHours'] as List<dynamic>?)
      ?.map((e) => OpeningHourDto.fromJson(e as Map<String, dynamic>))
      .toList(),
  holidays: (json['holidays'] as List<dynamic>?)
      ?.map((e) => HolidayDto.fromJson(e as Map<String, dynamic>))
      .toList(),
);

Map<String, dynamic> _$BranchDtoToJson(BranchDto instance) => <String, dynamic>{
  'irn': instance.irn,
  'id': instance.id,
  'name': instance.name,
  'latitude': instance.latitude,
  'longitude': instance.longitude,
  'url': instance.url,
  'opacUrl': instance.opacUrl,
  'imageUrl': instance.imageUrl,
  'phone': instance.phone,
  'email': instance.email,
  'address': instance.address,
  'openingHours': instance.openingHours,
  'holidays': instance.holidays,
};

SetInfo _$SetInfoFromJson(Map<String, dynamic> json) => SetInfo(
  setId: (json['setId'] as num?)?.toInt(),
  lastIrn: (json['lastIrn'] as num?)?.toInt(),
  setTotal: (json['setTotal'] as num?)?.toInt(),
  setCount: (json['setCount'] as num?)?.toInt(),
  hasMoreRecords: json['hasMoreRecords'] as bool?,
);

Map<String, dynamic> _$SetInfoToJson(SetInfo instance) => <String, dynamic>{
  'setId': instance.setId,
  'lastIrn': instance.lastIrn,
  'setTotal': instance.setTotal,
  'setCount': instance.setCount,
  'hasMoreRecords': instance.hasMoreRecords,
};

BranchResponseDto _$BranchResponseDtoFromJson(Map<String, dynamic> json) =>
    BranchResponseDto(
      setInfo: json['setInfo'] == null
          ? null
          : SetInfo.fromJson(json['setInfo'] as Map<String, dynamic>),
      branches: (json['branches'] as List<dynamic>?)
          ?.map((e) => BranchDto.fromJson(e as Map<String, dynamic>))
          .toList(),
    );

Map<String, dynamic> _$BranchResponseDtoToJson(BranchResponseDto instance) =>
    <String, dynamic>{
      'setInfo': instance.setInfo,
      'branches': instance.branches,
    };
