/// Dart mirrors of the middleware's rich-record JSON (`/record`, `/related`).
library;

class DetailValue {
  const DetailValue({required this.text, required this.url});
  final String text;
  final String? url;
  factory DetailValue.fromJson(Map<String, dynamic> j) =>
      DetailValue(text: j['text'] as String? ?? '', url: j['url'] as String?);
}

class DetailRow {
  const DetailRow({required this.label, required this.values});
  final String label;
  final List<DetailValue> values;
  factory DetailRow.fromJson(Map<String, dynamic> j) => DetailRow(
        label: j['label'] as String? ?? '',
        values: (j['values'] as List<dynamic>? ?? const [])
            .map((e) => DetailValue.fromJson(e as Map<String, dynamic>))
            .toList(growable: false),
      );
}

class RecordRelated {
  const RecordRelated({
    required this.similar,
    required this.author,
    required this.series,
    required this.versions,
  });
  final String? similar;
  final String? author;
  final String? series;
  final String? versions;
  factory RecordRelated.fromJson(Map<String, dynamic> j) => RecordRelated(
        similar: j['similar'] as String?,
        author: j['author'] as String?,
        series: j['series'] as String?,
        versions: j['versions'] as String?,
      );
}

class RecordDetail {
  const RecordDetail({
    required this.bibIrn,
    required this.brn,
    required this.title,
    required this.seriesTitle,
    required this.isbn,
    required this.coverUrls,
    required this.summary,
    required this.details,
    required this.bookmarkUrl,
    required this.related,
  });

  final String? bibIrn;
  final String? brn;
  final String? title;
  final String? seriesTitle;
  final String? isbn;
  final List<String> coverUrls;
  final String? summary;
  final List<DetailRow> details;
  final String? bookmarkUrl;
  final RecordRelated related;

  String? get coverUrl => coverUrls.isEmpty ? null : coverUrls.first;

  factory RecordDetail.fromJson(Map<String, dynamic> j) => RecordDetail(
        bibIrn: j['bibIrn'] as String?,
        brn: j['brn'] as String?,
        title: j['title'] as String?,
        seriesTitle: j['seriesTitle'] as String?,
        isbn: j['isbn'] as String?,
        coverUrls: (j['coverUrls'] as List<dynamic>? ?? const [])
            .map((e) => e as String)
            .toList(growable: false),
        summary: j['summary'] as String?,
        details: (j['details'] as List<dynamic>? ?? const [])
            .map((e) => DetailRow.fromJson(e as Map<String, dynamic>))
            .toList(growable: false),
        bookmarkUrl: j['bookmarkUrl'] as String?,
        related: RecordRelated.fromJson(
            (j['related'] as Map<String, dynamic>?) ?? const {}),
      );
}

class RelatedItem {
  const RelatedItem({
    required this.title,
    required this.bibUrl,
    required this.brn,
    required this.author,
    required this.year,
    required this.isbn,
    required this.coverUrls,
  });

  final String? title;
  final String? bibUrl;
  final String? brn;
  final String? author;
  final String? year;
  final String? isbn;
  final List<String> coverUrls;

  String? get coverUrl => coverUrls.isEmpty ? null : coverUrls.first;

  factory RelatedItem.fromJson(Map<String, dynamic> j) => RelatedItem(
        title: j['title'] as String?,
        bibUrl: j['bibUrl'] as String?,
        brn: j['brn'] as String?,
        author: j['author'] as String?,
        year: j['year'] as String?,
        isbn: j['isbn'] as String?,
        coverUrls: (j['coverUrls'] as List<dynamic>? ?? const [])
            .map((e) => e as String)
            .toList(growable: false),
      );
}

class RelatedList {
  const RelatedList({required this.items});
  final List<RelatedItem> items;
  factory RelatedList.fromJson(Map<String, dynamic> j) => RelatedList(
        items: (j['items'] as List<dynamic>? ?? const [])
            .map((e) => RelatedItem.fromJson(e as Map<String, dynamic>))
            .toList(growable: false),
      );
}
