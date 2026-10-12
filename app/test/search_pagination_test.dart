import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http_mock_adapter/http_mock_adapter.dart';
import 'package:spydus_clone/src/search/book_detail_screen.dart';
import 'package:spydus_clone/src/search/search_api.dart';
import 'package:spydus_clone/src/search/search_providers.dart';
import 'package:spydus_clone/src/search/search_screen.dart';

Map<String, dynamic> _record(String title) => {
      'rank': null,
      'workIrn': title,
      'title': title,
      'detailUrl': null,
      'author': 'Author of $title',
      'authorRole': 'author',
      'authorUrl': null,
      'year': '2025',
      'language': 'English',
      'summary': 'Summary of $title',
      'isbn': null,
      'coverUrls': <String>[], // empty → placeholder, no network in tests
      'formats': [
        {'code': 'BK', 'label': 'Book', 'reserveUrl': null, 'availabilityUrl': null},
      ],
    };

Map<String, dynamic> _page(List<String> titles, {String? next}) => {
      'query': 'bsc',
      'setId': 'SET1',
      'totalResults': 20,
      'showing': {'from': 1, 'to': 10},
      'nextPageUrl': next,
      'prevPageUrl': null,
      'records': titles.map(_record).toList(),
    };

void main() {
  late Dio dio;
  late DioAdapter adapter;
  const nextUrl = 'https://canning.spydus.com/cgi-bin/spydus.exe/SET/OPAC/ALLWRKENQ/SET1/999?NREC=10';

  setUp(() {
    dio = Dio(BaseOptions(baseUrl: 'http://test.local'));
    adapter = DioAdapter(dio: dio);
    adapter.onGet(
      '/search',
      (s) => s.reply(200, _page(List.generate(10, (i) => 'P1 Book ${i + 1}'), next: nextUrl)),
      queryParameters: {'q': 'bsc'},
    );
    adapter.onGet(
      '/page',
      (s) => s.reply(200, _page(List.generate(10, (i) => 'P2 Book ${i + 1}'), next: null)),
      queryParameters: {'url': nextUrl},
    );
  });

  Widget harness() => ProviderScope(
        overrides: [middlewareApiProvider.overrideWithValue(MiddlewareApi(dio: dio))],
        child: const MaterialApp(home: SearchScreen()),
      );

  testWidgets('submitting a query shows page 1', (tester) async {
    await tester.pumpWidget(harness());
    await tester.enterText(find.byType(TextField), 'bsc');
    await tester.tap(find.widgetWithText(FilledButton, 'Search'));
    await tester.pumpAndSettle();

    expect(find.text('Showing 10 of 20'), findsOneWidget);
    expect(find.text('P1 Book 1'), findsOneWidget);
    expect(find.text('P2 Book 1'), findsNothing);
  });

  testWidgets('infinite scroll fetches and appends the next page', (tester) async {
    await tester.pumpWidget(harness());
    await tester.enterText(find.byType(TextField), 'bsc');
    await tester.tap(find.widgetWithText(FilledButton, 'Search'));
    await tester.pumpAndSettle();

    // Scroll the results list to the bottom to trigger the pager.
    await tester.drag(find.byType(ListView), const Offset(0, -6000));
    await tester.pumpAndSettle();

    // Page 2 has been fetched and appended (10 → 20 records).
    expect(find.text('Showing 20 of 20'), findsOneWidget);

    // The last record is lazily built; scroll to it to confirm it's real.
    await tester.scrollUntilVisible(
      find.text('P2 Book 10'),
      400,
      scrollable: find.byType(Scrollable).last,
    );
    expect(find.text('P2 Book 10'), findsOneWidget);
  });

  testWidgets('tapping a result opens the book detail screen', (tester) async {
    await tester.pumpWidget(harness());
    await tester.enterText(find.byType(TextField), 'bsc');
    await tester.tap(find.widgetWithText(FilledButton, 'Search'));
    await tester.pumpAndSettle();

    await tester.tap(find.text('P1 Book 1'));
    await tester.pumpAndSettle();

    expect(find.byType(BookDetailScreen), findsOneWidget);
    expect(find.text('Details'), findsOneWidget);
    expect(find.text('Summary of P1 Book 1'), findsOneWidget);
  });
}
