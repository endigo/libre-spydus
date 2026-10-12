import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http_mock_adapter/http_mock_adapter.dart';
import 'package:spydus_clone/src/search/book_detail_screen.dart';
import 'package:spydus_clone/src/search/search_api.dart';
import 'package:spydus_clone/src/search/search_models.dart';
import 'package:spydus_clone/src/search/search_providers.dart';

const _ebkUrl = 'https://canning.spydus.com/cgi-bin/spydus.exe/ENQ/OPAC/BIBENQ/78552136';

// A pure-ebook record: EBK format, no reserveUrl (ebooks aren't reservable via
// the OPAC branch-pickup flow).
SearchRecord _ebookRecord() => SearchRecord.fromJson({
      'rank': 1,
      'workIrn': '1',
      'title': 'Atomic Habits',
      'detailUrl': null,
      'author': 'Clear, James',
      'authorRole': 'author',
      'authorUrl': null,
      'year': '2018',
      'language': 'English',
      'summary': null,
      'isbn': null,
      'coverUrls': <String>[],
      'formats': [
        {'code': 'EBK', 'label': 'eBook', 'reserveUrl': null, 'availabilityUrl': _ebkUrl},
      ],
    });

void main() {
  late Dio dio;
  late DioAdapter adapter;

  setUp(() {
    dio = Dio(BaseOptions(baseUrl: 'http://test.local'));
    adapter = DioAdapter(dio: dio);
    // Ebooks report no branch counts.
    adapter.onGet('/availability', (s) => s.reply(200, {
          'bibRecordIrn': null, 'title': null, 'publisher': null,
          'counts': <String, dynamic>{}, 'reserved': null, 'copies': null,
          'holdingsUrl': null, 'recordUrl': null,
        }), queryParameters: {'url': _ebkUrl});
    adapter.onGet('/record', (s) => s.reply(200, {
          'bibIrn': '1', 'brn': null, 'title': 'Atomic Habits', 'seriesTitle': null, 'isbn': null,
          'coverUrls': <String>[], 'summary': null, 'details': <dynamic>[], 'bookmarkUrl': null,
          'related': {'similar': null, 'author': null, 'series': null, 'versions': null},
        }), queryParameters: {'url': _ebkUrl});
  });

  Widget harness() => ProviderScope(
        overrides: [middlewareApiProvider.overrideWithValue(MiddlewareApi(dio: dio))],
        child: MaterialApp(home: BookDetailScreen(record: _ebookRecord())),
      );

  testWidgets('an ebook shows an online title, not branches or a reserve button', (tester) async {
    await tester.pumpWidget(harness());
    await tester.pumpAndSettle();

    // Digital wording, not branch wording.
    expect(find.text('Available online'), findsOneWidget);
    expect(find.text('See branch availability'), findsNothing);

    // No OPAC branch-pickup reservation for ebooks.
    expect(find.widgetWithText(FilledButton, 'Place reservation'), findsNothing);
    expect(find.widgetWithText(FilledButton, 'Sign in to reserve'), findsNothing);
  });
}
