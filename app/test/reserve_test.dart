import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http_mock_adapter/http_mock_adapter.dart';
import 'package:spydus_clone/src/search/book_detail_screen.dart';
import 'package:spydus_clone/src/search/search_api.dart';
import 'package:spydus_clone/src/search/search_models.dart';
import 'package:spydus_clone/src/search/search_providers.dart';

const _availUrl = 'https://canning.spydus.com/avail/BK';
const _reserveUrl = 'https://canning.spydus.com/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/1/0/V/0/R/9/ALLWRKENQ?SVL=1&RECFMT=BK';

SearchRecord _record() => SearchRecord.fromJson({
      'rank': 1,
      'workIrn': '1',
      'title': 'Jessi Ramsey, pet-sitter',
      'detailUrl': null,
      'author': 'Crenshaw, Ellen T.',
      'authorRole': 'author',
      'authorUrl': null,
      'year': '2025',
      'language': 'English',
      'summary': null,
      'isbn': null,
      'coverUrls': <String>[],
      'formats': [
        {'code': 'BK', 'label': 'Book', 'reserveUrl': _reserveUrl, 'availabilityUrl': _availUrl},
      ],
    });

void main() {
  final secureStore = <String, String>{'spydus_opac_session': 'SESSION-123'};
  const channel = MethodChannel('plugins.it_nomads.com/flutter_secure_storage');

  setUpAll(() {
    TestWidgetsFlutterBinding.ensureInitialized();
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
      final key = call.arguments is Map ? call.arguments['key'] as String? : null;
      switch (call.method) {
        case 'read':
          return secureStore[key];
        case 'write':
          secureStore[key!] = call.arguments['value'] as String;
          return null;
        case 'delete':
          secureStore.remove(key);
          return null;
        case 'readAll':
          return Map<String, String>.from(secureStore);
      }
      return null;
    });
  });

  late Dio dio;
  late DioAdapter adapter;
  var placeCalls = 0;

  setUp(() {
    placeCalls = 0;
    dio = Dio(BaseOptions(baseUrl: 'http://test.local'));
    adapter = DioAdapter(dio: dio);
    adapter.onGet('/availability',
        (s) => s.reply(200, {
              'bibRecordIrn': '1', 'title': 'x', 'publisher': null,
              'counts': {'copies': 3, 'reserved': 1}, 'reserved': 1, 'copies': 3,
              'holdingsUrl': null, 'recordUrl': null,
            }),
        queryParameters: {'url': _availUrl});
    adapter.onGet('/record', (s) => s.reply(200, {
          'bibIrn': '1', 'brn': '1', 'title': 'Jessi', 'seriesTitle': null, 'isbn': null,
          'coverUrls': <String>[], 'summary': null, 'details': <dynamic>[], 'bookmarkUrl': null,
          'related': {'similar': null, 'author': null, 'series': null, 'versions': null},
        }), queryParameters: {'url': _availUrl});
    adapter.onGet('/reserve/options',
        (s) => s.reply(200, {
              'canReserve': true,
              'pickupLocation': 'RIVERTON LIBRARY',
              'availableSummary': 'Canning - Reservable items 3, available 0',
              'expiryChoices': [
                {'days': '0', 'label': 'No expiry'},
                {'days': '30', 'label': '1 month'},
              ],
              'message': null,
            }),
        queryParameters: {'url': _reserveUrl});
    adapter.onPost('/reserve', (s) {
      placeCalls++;
      return s.reply(200, {'ok': true, 'message': 'Reservation placed.'});
    }, data: {'url': _reserveUrl, 'xdays': '0'});
  });

  Widget harness() => ProviderScope(
        overrides: [middlewareApiProvider.overrideWithValue(MiddlewareApi(dio: dio))],
        child: MaterialApp(home: BookDetailScreen(record: _record())),
      );

  testWidgets('signed-in user can place a reservation from the detail screen', (tester) async {
    await tester.pumpWidget(harness());
    await tester.pumpAndSettle();

    // The availability card shows the reserve action (signed in via mock store).
    final reserveBtn = find.widgetWithText(FilledButton, 'Place reservation');
    expect(reserveBtn, findsOneWidget);

    await tester.tap(reserveBtn);
    await tester.pumpAndSettle();

    // The sheet loaded the form: pickup location + confirm button.
    expect(find.text('RIVERTON LIBRARY'), findsOneWidget);
    expect(find.widgetWithText(FilledButton, 'Confirm reservation'), findsOneWidget);

    await tester.tap(find.widgetWithText(FilledButton, 'Confirm reservation'));
    await tester.pumpAndSettle();

    // The hold was placed and a confirmation is shown.
    expect(placeCalls, 1);
    expect(find.text('Reservation placed.'), findsOneWidget);
  });
}
