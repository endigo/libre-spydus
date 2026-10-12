import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http_mock_adapter/http_mock_adapter.dart';
import 'package:spydus_clone/src/account/account_screen.dart';
import 'package:spydus_clone/src/search/search_api.dart';
import 'package:spydus_clone/src/search/search_providers.dart';

const _cancelUrl =
    'https://canning.spydus.com/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78543769/0/C/5/RSVCENQ?SVL=19387718';
const _editUrl =
    'https://canning.spydus.com/cgi-bin/spydus.exe/PGM/OPAC/CCOPT/78543769/0/ER/1/RSVCENQ?SVL=19387718';

// The mocked endpoints return distinctive messages. Because `/reserve/cancel`
// and `/reserve/edit` are registered with an exact `data:` matcher, these
// messages only render when the app actually POSTs with the expected body —
// so asserting on them proves both "the request fired" and "with the right
// fields" (http_mock_adapter runs the reply callback at registration time, so
// a hit counter there would not measure requests).
Map<String, dynamic> _reservationsJson() => {
      'ready': {'total': 0, 'items': <dynamic>[]},
      'pending': {
        'total': 1,
        'items': [
          {
            'irn': '19387718',
            'bibIrn': '771163',
            'title': 'Get programming with JavaScript',
            'bibUrl': null,
            'author': 'Larsen, John',
            'year': '2016',
            'coverUrls': <String>[],
            'isbn': null,
            'fields': {'Status': 'Requested pickup location: RIVERTON LIBRARY'},
            'actions': [
              {'label': 'Cancel reservation', 'href': _cancelUrl},
              {'label': 'Edit reservation', 'href': _editUrl},
            ],
          },
        ],
      },
    };

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
        case 'readAll':
          return Map<String, String>.from(secureStore);
      }
      return null;
    });
  });

  late Dio dio;
  late DioAdapter adapter;

  setUp(() {
    dio = Dio(BaseOptions(baseUrl: 'http://test.local'));
    adapter = DioAdapter(dio: dio);
    adapter.onGet('/account/reservations', (s) => s.reply(200, _reservationsJson()));
    adapter.onPost('/reserve/cancel', (s) => s.reply(200, {'ok': true, 'message': 'Reservation cancelled.'}),
        data: {'url': _cancelUrl});
    adapter.onGet('/reserve/edit', (s) => s.reply(200, {
          'canEdit': true,
          'pickupChoices': [
            {'code': '25353', 'name': 'CANNINGTON LIBRARY'},
            {'code': '25363', 'name': 'RIVERTON LIBRARY'},
            {'code': '25379', 'name': 'WILLETTON LIBRARY'},
          ],
          'pickupSelected': null,
          'delayUntil': null,
          'notRequiredAfter': null,
          'message': null,
        }), queryParameters: {'url': _editUrl});
    // The current pickup (RIVERTON = 25363) is pre-selected and resubmitted.
    adapter.onPost('/reserve/edit', (s) => s.reply(200, {'ok': true, 'message': 'Reservation updated.'}),
        data: {'url': _editUrl, 'pickup': '25363'});
  });

  Widget harness() => ProviderScope(
        overrides: [middlewareApiProvider.overrideWithValue(MiddlewareApi(dio: dio))],
        child: const MaterialApp(home: MyReservationsScreen()),
      );

  testWidgets('cancelling asks for confirmation, then reports success', (tester) async {
    await tester.pumpWidget(harness());
    await tester.pumpAndSettle();
    expect(find.text('Get programming with JavaScript'), findsOneWidget);

    await tester.tap(find.byIcon(Icons.more_vert));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();

    // A confirmation dialog gates the destructive action; nothing sent yet.
    expect(find.text('Cancel reservation?'), findsOneWidget);
    expect(find.text('Reservation cancelled.'), findsNothing);

    await tester.tap(find.text('Cancel hold'));
    await tester.pumpAndSettle();

    // Success message only renders if the POST fired with {url: cancelUrl}.
    expect(find.text('Reservation cancelled.'), findsOneWidget);
  });

  testWidgets('dismissing the confirmation does not cancel', (tester) async {
    await tester.pumpWidget(harness());
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.more_vert));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Keep it'));
    await tester.pumpAndSettle();

    expect(find.text('Cancel reservation?'), findsNothing);
    expect(find.text('Reservation cancelled.'), findsNothing);
  });

  testWidgets('editing pre-selects the current pickup and saves', (tester) async {
    await tester.pumpWidget(harness());
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.more_vert));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Edit'));
    await tester.pumpAndSettle();

    // The sheet loaded with the hold's current location pre-selected.
    expect(find.text('Edit reservation'), findsOneWidget);
    expect(find.text('RIVERTON LIBRARY'), findsOneWidget);

    await tester.tap(find.widgetWithText(FilledButton, 'Save changes'));
    await tester.pumpAndSettle();

    // Success only renders if the POST fired with {url, pickup: 25363}.
    expect(find.text('Reservation updated.'), findsOneWidget);
  });
}
