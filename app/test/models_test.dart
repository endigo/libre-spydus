import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:spydus_clone/src/models/models.dart';

void main() {
  group('LibraryDto (real directory shape)', () {
    test('parses the public library list fixture', () {
      final raw = File('test/fixtures/libraries_sample.json').readAsStringSync();
      final list = (jsonDecode(raw) as List)
          .map((e) => LibraryDto.fromJson(e as Map<String, dynamic>))
          .toList();

      expect(list, hasLength(3));
      final canning = list.first;
      expect(canning.name, 'City of Canning');
      expect(canning.url, 'https://canning.spydus.com');
      expect(canning.location?.country, 'Australia');
      expect(canning.location?.latitude, closeTo(-32.0396, 0.0001));
      expect(canning.pinningHash, isNull);
    });
  });

  group('Token responses have different wire casing', () {
    test('RefreshTokenResponse (code exchange) uses snake_case', () {
      final r = RefreshTokenResponse.fromJson(
        {'access_token': 'AAA', 'refresh_token': 'RRR'},
      );
      expect(r.accessToken, 'AAA');
      expect(r.refreshToken, 'RRR');
      expect(r.hasTokens, isTrue);
    });

    test('AccessTokenResponse (refresh) uses camelCase', () {
      final r = AccessTokenResponse.fromJson(
        {'accessToken': 'AAA', 'refreshToken': 'RRR'},
      );
      expect(r.accessToken, 'AAA');
      expect(r.refreshToken, 'RRR');
      expect(r.hasTokens, isTrue);
    });
  });

  group('StyleConfigDto', () {
    test('parses config incl. WiFi-only flags seen on the wire', () {
      final cfg = StyleConfigDto.fromJson({
        'barcodeType': 'CODE128',
        'selfIssueEnabled': true,
        'selfReturnEnabled': false,
        'selfIssueWiFiOnly': true,
        'selfReturnWiFiOnly': false,
        'enableJoinOnline': true,
        'langs': [
          {'desc': 'English', 'id': 'ENG', 'iso': 'en-AU'}
        ],
        'style': {'primary': '#009CA6'},
        'discovery': {'searchType': 'KW', 'defaultSort': 'RELEVANCE'},
        'showNearbyBranches': true,
      });
      expect(cfg.barcodeType, 'CODE128');
      expect(cfg.selfIssueEnabled, isTrue);
      expect(cfg.selfIssueWiFiOnly, isTrue);
      expect(cfg.langs?.first.languageName, 'en');
      expect(cfg.style?['primary'], '#009CA6');
      expect(cfg.discovery?.searchType, 'KW');
    });
  });

  group('NotificationDto / session claims mapping', () {
    test('maps brwdIrn/brwIrn/ntfId correctly', () {
      final n = NotificationDto.fromJson({
        'brwdIrn': 123,
        'brwIrn': 456,
        'ntfId': '11111111-2222-3333-4444-555555555555',
        'isRead': false,
        'subject': 'Item due',
        'message': 'Your loan is due soon',
      });
      expect(n.borrowerConsortiumRecordIdentifier, 123);
      expect(n.borrowerLocalAccountIdentifier, 456);
      expect(n.id, '11111111-2222-3333-4444-555555555555');
      expect(n.isRead, isFalse);
    });

    test('ValidateSessionResponse maps opaque borrower fields', () {
      final v = ValidateSessionResponse.fromJson({
        'institutionId': 'WQUP',
        'name': 'Jane Reader',
        'barcode': 'B000',
        'brwIrn': '456',
        'brwdIrn': '123',
        'brwSessionId': 'sess-xyz',
      });
      expect(v.institutionId, 'WQUP');
      expect(v.borrowerLocalAccountIdentifier, '456');
      expect(v.borrowerConsortiumRecordIdentifier, '123');
      expect(v.borrowerSessionId, 'sess-xyz');
    });
  });
}
