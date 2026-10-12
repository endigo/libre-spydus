import 'dart:convert';

import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http_mock_adapter/http_mock_adapter.dart';
import 'package:spydus_clone/src/api/catalog_client.dart';
import 'package:spydus_clone/src/api/discovery_client.dart';
import 'package:spydus_clone/src/api/patron_client.dart';
import 'package:spydus_clone/src/models/models.dart';

/// These tests assert that the generated Retrofit clients build exactly the
/// request (method, path, query, headers, body) that the recovered contract
/// specifies. http_mock_adapter only replies when the request matches the
/// registered route, so a mismatch fails the call.
void main() {
  late Dio dio;
  late DioAdapter adapter;

  setUp(() {
    dio = Dio(BaseOptions(baseUrl: 'https://canning.spydus.com'));
    adapter = DioAdapter(dio: dio);
  });

  test('DiscoveryClient.getLibraries → GET /mobile/libraries?groupCode=', () {
    final d = Dio(BaseOptions(baseUrl: 'https://librarylist.spydus.com/api/1.0'));
    final a = DioAdapter(dio: d);
    a.onGet(
      '/mobile/libraries',
      (s) => s.reply(200, [
        {'name': 'City of Canning', 'id': 'x', 'url': 'https://canning.spydus.com'}
      ]),
      queryParameters: {'groupCode': 'GRP'},
    );
    final client = DiscoveryClient(d);
    expect(
      client.getLibraries(groupCode: 'GRP').then((r) => r.first.url),
      completion('https://canning.spydus.com'),
    );
  });

  test('CatalogClient.search → GET /api/mobile/1.0/discover/search?keywords=', () {
    adapter.onGet(
      '/api/mobile/1.0/discover/search',
      (s) => s.reply(200, '"https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/..."'),
      queryParameters: {'keywords': 'Harry Potter'},
      headers: {'spydus-lang': ''},
    );
    final client = CatalogClient(dio);
    expect(
      client.search(keywords: 'Harry Potter', spydusLang: ''),
      completion(contains('cgi-bin/spydus.exe')),
    );
  });

  test('CatalogClient.getBranches → GET with setId/lastIrn/count', () {
    adapter.onGet(
      '/api/mobile/1.0/mobile/branches',
      (s) => s.reply(200, {
        'setInfo': {'setId': 0, 'lastIrn': 0, 'setTotal': 1, 'setCount': 1, 'hasMoreRecords': false},
        'branches': [
          {'irn': 1, 'id': 'MAIN', 'name': 'Canning Library'}
        ],
      }),
      queryParameters: {'setId': 0, 'lastIrn': 0, 'count': 500},
      headers: {'spydus-lang': ''},
    );
    final client = CatalogClient(dio);
    expect(
      client.getBranches(setId: 0, lastIrn: 0, count: 500, spydusLang: '')
          .then((r) => r.branches?.first.name),
      completion('Canning Library'),
    );
  });

  test('CatalogClient.validateSession → POST body {sessionId}', () {
    adapter.onPost(
      '/api/private/1.0/patrons/validatesessionid',
      (s) => s.reply(200, {
        'institutionId': 'WQUP',
        'name': 'Reader',
        'barcode': 'B1',
        'brwIrn': '456',
        'brwdIrn': '123',
        'brwSessionId': 'sess',
      }),
      data: {'sessionId': 'sess'},
    );
    final client = CatalogClient(dio);
    expect(
      client.validateSession(ValidateSessionRequest(borrowerSessionId: 'sess'))
          .then((r) => r.borrowerConsortiumRecordIdentifier),
      completion('123'),
    );
  });

  test('PatronClient.getNotifications → path uses brwdIrn', () {
    adapter.onGet(
      '/api/mobile/1.0/patrons/123/notifications',
      (s) => s.reply(200, {
        'notifications': [
          {'ntfId': 'g', 'subject': 'Due', 'isRead': false}
        ],
        'totalCount': 1,
      }),
    );
    final client = PatronClient(dio);
    expect(
      client.getNotifications(123).then((r) => r.totalCount),
      completion(1),
    );
  });

  test('config endpoint returns a raw JSON string that parses to StyleConfigDto', () {
    final cfgString = jsonEncode({'barcodeType': 'CODE128', 'selfIssueEnabled': true});
    adapter.onGet(
      '/api/mobile/1.0/general/config',
      (s) => s.reply(200, cfgString),
      queryParameters: {'wifiName': ''},
    );
    final client = CatalogClient(dio);
    // The endpoint returns the config as a raw string body (ResponseType.plain);
    // SpydusApi.styleConfig() is what jsonDecodes it. Here we assert the client
    // surfaces the raw JSON string intact.
    expect(
      client.getStyleConfiguration(wifiName: '').then((raw) => raw.contains('CODE128')),
      completion(isTrue),
    );
  });
}
