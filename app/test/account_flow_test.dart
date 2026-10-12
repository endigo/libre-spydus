import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http_mock_adapter/http_mock_adapter.dart';
import 'package:spydus_clone/src/account/account_screen.dart';
import 'package:spydus_clone/src/search/search_api.dart';
import 'package:spydus_clone/src/search/search_providers.dart';

Map<String, dynamic> _item(String title, {Map<String, String>? fields}) => {
      'irn': title,
      'title': title,
      'bibUrl': null,
      'author': 'Author $title',
      'year': '2025',
      'coverUrls': <String>[],
      'isbn': null,
      'fields': fields ?? <String, String>{},
      'actions': <Map<String, dynamic>>[],
    };

void main() {
  // In-memory mock for flutter_secure_storage's method channel.
  final secureStore = <String, String>{};
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
        case 'deleteAll':
          secureStore.clear();
          return null;
      }
      return null;
    });
  });

  late Dio dio;
  late DioAdapter adapter;

  setUp(() {
    secureStore.clear();
    dio = Dio(BaseOptions(baseUrl: 'http://test.local'));
    adapter = DioAdapter(dio: dio);
    adapter.onPost(
      '/login',
      (s) => s.reply(200, {'ok': true, 'session': 'SESSION-123'}),
      data: {'username': 'CARD1', 'password': 'PIN1'},
    );
    adapter.onGet(
      '/account/loans',
      (s) => s.reply(200, {
        'total': 1,
        'items': [_item('Bluestar\'s prophecy', fields: {'Due': '27 Oct 2026'})],
      }),
    );
    adapter.onGet(
      '/account/reservations',
      (s) => s.reply(200, {
        'ready': {
          'total': 1,
          'items': [_item('Kristy\'s big day', fields: {'Status': 'Please pickup from: RIVERTON LIBRARY'})],
        },
        'pending': {
          'total': 1,
          'items': [_item('Peppa Pig story treasury', fields: {'Status': 'Requested pickup: RIVERTON'})],
        },
      }),
    );
  });

  Widget harness() => ProviderScope(
        overrides: [middlewareApiProvider.overrideWithValue(MiddlewareApi(dio: dio))],
        child: const MaterialApp(home: AccountScreen()),
      );

  testWidgets('full account flow: sign in → loans → reservations', (tester) async {
    await tester.pumpWidget(harness());
    await tester.pumpAndSettle();

    // Signed out initially.
    expect(find.widgetWithText(FilledButton, 'Sign in'), findsOneWidget);

    // Open login, enter credentials, submit.
    await tester.tap(find.widgetWithText(FilledButton, 'Sign in'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField).first, 'CARD1');
    await tester.enterText(find.byType(TextField).last, 'PIN1');
    await tester.tap(find.widgetWithText(FilledButton, 'Sign in'));
    await tester.pumpAndSettle();

    // Back on the account hub, now signed in.
    expect(find.text('My loans'), findsOneWidget);
    expect(find.text('My reservations'), findsOneWidget);
    expect(find.text('Sign out'), findsOneWidget);
    // Session persisted to (mock) secure storage.
    expect(secureStore.values, contains('SESSION-123'));

    // My loans.
    await tester.tap(find.text('My loans'));
    await tester.pumpAndSettle();
    expect(find.text("Bluestar's prophecy"), findsOneWidget);
    expect(find.textContaining('Due 27 Oct 2026'), findsOneWidget);

    await tester.pageBack();
    await tester.pumpAndSettle();

    // My reservations: both sections present.
    await tester.tap(find.text('My reservations'));
    await tester.pumpAndSettle();
    expect(find.text('Ready for pickup'), findsOneWidget);
    expect(find.text('On the way'), findsOneWidget);
    expect(find.text("Kristy's big day"), findsOneWidget);
    expect(find.text('Peppa Pig story treasury'), findsOneWidget);
  });
}
