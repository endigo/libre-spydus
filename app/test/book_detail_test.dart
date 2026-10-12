import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http_mock_adapter/http_mock_adapter.dart';
import 'package:spydus_clone/src/search/book_detail_screen.dart';
import 'package:spydus_clone/src/search/search_api.dart';
import 'package:spydus_clone/src/search/search_providers.dart';

const _bibUrl = 'https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/1?BRN=442829&SETLVL=';
const _seriesUrl = 'https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/2?GCN=SERIES';
const _siblingUrl = 'https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/3?BRN=461008&SETLVL=';

Map<String, dynamic> _detail(String title, {String? series}) => {
      'bibIrn': '17651573',
      'brn': '442829',
      'title': title,
      'seriesTitle': 'Baby-sitters Club (Graphix (Firm)) ; 18.',
      'isbn': '9781761725456',
      'coverUrls': <String>[],
      'summary': null,
      'details': [
        {
          'label': 'Imprint',
          'values': [
            {'text': '[Gosford, NSW] : Scholastic Australia, 2025.', 'url': null},
          ],
        },
        {
          'label': 'Series title',
          'values': [
            {'text': 'Baby-sitters Club (Graphix (Firm)) ; 18.', 'url': 'http://x'},
          ],
        },
      ],
      'bookmarkUrl': 'https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ?SETLVL=&BRN=442829',
      'related': {
        'similar': null,
        'author': null,
        'series': series,
        'versions': null,
      },
    };

Map<String, dynamic> _related(List<String> titles) => {
      'items': titles
          .map((t) => {
                'title': t,
                'bibUrl': _siblingUrl,
                'brn': '461008',
                'author': 'Author of $t',
                'year': '2025',
                'isbn': null,
                'coverUrls': <String>[],
              })
          .toList(),
    };

void main() {
  late Dio dio;
  late DioAdapter adapter;

  setUp(() {
    dio = Dio(BaseOptions(baseUrl: 'http://test.local'));
    adapter = DioAdapter(dio: dio);
    adapter.onGet('/record',
        (s) => s.reply(200, _detail('Jessi Ramsey, pet-sitter', series: _seriesUrl)),
        queryParameters: {'url': _bibUrl});
    adapter.onGet('/related', (s) => s.reply(200, _related(['BSC 13', 'BSC 14'])),
        queryParameters: {'url': _seriesUrl});
    // The sibling opened by tapping a related card (distinct title proves nav).
    adapter.onGet('/record', (s) => s.reply(200, _detail('BSC 13 FULL RECORD', series: null)),
        queryParameters: {'url': _siblingUrl});
  });

  Widget harness() => ProviderScope(
        overrides: [middlewareApiProvider.overrideWithValue(MiddlewareApi(dio: dio))],
        child: const MaterialApp(
          home: BookDetailScreen.fromUrl(recordUrl: _bibUrl, title: 'Jessi Ramsey, pet-sitter'),
        ),
      );

  testWidgets('renders rich details and the in-series carousel', (tester) async {
    await tester.pumpWidget(harness());
    await tester.pumpAndSettle();

    // Rich record-details block.
    expect(find.text('Record details'), findsOneWidget);
    expect(find.text('Imprint'), findsOneWidget);
    expect(find.text('[Gosford, NSW] : Scholastic Australia, 2025.'), findsOneWidget);

    // In-series panel loaded its cards.
    expect(find.text('In this series'), findsOneWidget);
    expect(find.text('BSC 13'), findsOneWidget);
    expect(find.text('BSC 14'), findsOneWidget);
  });

  testWidgets('tapping a related title opens its detail', (tester) async {
    await tester.pumpWidget(harness());
    await tester.pumpAndSettle();

    // The in-series strip is below the fold; bring it on-screen, then tap.
    await tester.ensureVisible(find.text('BSC 13'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('BSC 13'));
    await tester.pumpAndSettle();

    // The sibling's detail loaded on a new screen (its distinct title shows).
    expect(find.text('BSC 13 FULL RECORD'), findsOneWidget);
  });
}
