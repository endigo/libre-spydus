import 'package:json_annotation/json_annotation.dart';

part 'self_issue.g.dart';

/// Mirrors `Spydus.Domain.DTO.SelfIssueAlertDto`.
@JsonSerializable()
class SelfIssueAlertDto {
  final String? code;
  final bool? critical;
  final int? irn;
  final String? ref;
  final String? text;

  SelfIssueAlertDto({this.code, this.critical, this.irn, this.ref, this.text});

  factory SelfIssueAlertDto.fromJson(Map<String, dynamic> json) => _$SelfIssueAlertDtoFromJson(json);
  Map<String, dynamic> toJson() => _$SelfIssueAlertDtoToJson(this);
}

/// Mirrors `Spydus.Domain.DTO.SelfIssueResponseDto`
/// (self issue / RFID issue / self return responses).
@JsonSerializable()
class SelfIssueResponseDto {
  final List<SelfIssueAlertDto>? alerts;
  final String? bibAuthor;
  final String? bibIssueTitle;
  final String? bibTitle;
  final String? coverImage;
  final DateTime? dueDate;
  final bool? isIssued;
  final bool? itemCheckedIn;
  final String? itemId;
  final int? itemIrn;
  final String? itemRfid;
  final int? loanIrn;
  final int? patronIrn;
  final double? rentalAmount;

  SelfIssueResponseDto({
    this.alerts,
    this.bibAuthor,
    this.bibIssueTitle,
    this.bibTitle,
    this.coverImage,
    this.dueDate,
    this.isIssued,
    this.itemCheckedIn,
    this.itemId,
    this.itemIrn,
    this.itemRfid,
    this.loanIrn,
    this.patronIrn,
    this.rentalAmount,
  });

  factory SelfIssueResponseDto.fromJson(Map<String, dynamic> json) =>
      _$SelfIssueResponseDtoFromJson(json);
  Map<String, dynamic> toJson() => _$SelfIssueResponseDtoToJson(this);
}
