/// Dart mirrors of the middleware's JSON contract (`middleware/src/types.ts`).
/// Hand-written `fromJson` — these are simple DTOs and don't need codegen.
library;

class SearchResults {
  const SearchResults({
    required this.query,
    required this.setId,
    required this.totalResults,
    required this.showing,
    required this.nextPageUrl,
    required this.prevPageUrl,
    required this.records,
  });

  final String? query;
  final String? setId;
  final int? totalResults;
  final ShowingRange? showing;
  final String? nextPageUrl;
  final String? prevPageUrl;
  final List<SearchRecord> records;

  factory SearchResults.fromJson(Map<String, dynamic> j) => SearchResults(
        query: j['query'] as String?,
        setId: j['setId'] as String?,
        totalResults: j['totalResults'] as int?,
        showing: j['showing'] == null
            ? null
            : ShowingRange.fromJson(j['showing'] as Map<String, dynamic>),
        nextPageUrl: j['nextPageUrl'] as String?,
        prevPageUrl: j['prevPageUrl'] as String?,
        records: (j['records'] as List<dynamic>? ?? const [])
            .map((e) => SearchRecord.fromJson(e as Map<String, dynamic>))
            .toList(growable: false),
      );
}

class ShowingRange {
  const ShowingRange({required this.from, required this.to});
  final int from;
  final int to;
  factory ShowingRange.fromJson(Map<String, dynamic> j) =>
      ShowingRange(from: j['from'] as int, to: j['to'] as int);
}

class SearchRecord {
  const SearchRecord({
    required this.rank,
    required this.workIrn,
    required this.title,
    required this.detailUrl,
    required this.author,
    required this.authorRole,
    required this.authorUrl,
    required this.year,
    required this.language,
    required this.summary,
    required this.isbn,
    required this.coverUrls,
    required this.formats,
  });

  final int? rank;
  final String? workIrn;
  final String? title;
  final String? detailUrl;
  final String? author;
  final String? authorRole;
  final String? authorUrl;
  final String? year;
  final String? language;
  final String? summary;
  final String? isbn;
  final List<String> coverUrls;
  final List<RecordFormat> formats;

  /// First usable cover image, if any.
  String? get coverUrl => coverUrls.isEmpty ? null : coverUrls.first;

  factory SearchRecord.fromJson(Map<String, dynamic> j) => SearchRecord(
        rank: j['rank'] as int?,
        workIrn: j['workIrn'] as String?,
        title: j['title'] as String?,
        detailUrl: j['detailUrl'] as String?,
        author: j['author'] as String?,
        authorRole: j['authorRole'] as String?,
        authorUrl: j['authorUrl'] as String?,
        year: j['year'] as String?,
        language: j['language'] as String?,
        summary: j['summary'] as String?,
        isbn: j['isbn'] as String?,
        coverUrls: (j['coverUrls'] as List<dynamic>? ?? const [])
            .map((e) => e as String)
            .toList(growable: false),
        formats: (j['formats'] as List<dynamic>? ?? const [])
            .map((e) => RecordFormat.fromJson(e as Map<String, dynamic>))
            .toList(growable: false),
      );
}

class RecordFormat {
  const RecordFormat({
    required this.code,
    required this.label,
    required this.reserveUrl,
    required this.availabilityUrl,
  });

  final String code;
  final String? label;
  final String? reserveUrl;
  final String? availabilityUrl;

  factory RecordFormat.fromJson(Map<String, dynamic> j) => RecordFormat(
        code: j['code'] as String,
        label: j['label'] as String?,
        reserveUrl: j['reserveUrl'] as String?,
        availabilityUrl: j['availabilityUrl'] as String?,
      );
}

class Availability {
  const Availability({
    required this.bibRecordIrn,
    required this.title,
    required this.publisher,
    required this.counts,
    required this.reserved,
    required this.copies,
    required this.holdingsUrl,
    required this.recordUrl,
  });

  final String? bibRecordIrn;
  final String? title;
  final String? publisher;
  final Map<String, int> counts;
  final int? reserved;
  final int? copies;
  final String? holdingsUrl;
  final String? recordUrl;

  bool get hasCounts => copies != null || reserved != null;

  factory Availability.fromJson(Map<String, dynamic> j) => Availability(
        bibRecordIrn: j['bibRecordIrn'] as String?,
        title: j['title'] as String?,
        publisher: j['publisher'] as String?,
        counts: (j['counts'] as Map<String, dynamic>? ?? const {})
            .map((k, v) => MapEntry(k, (v as num).toInt())),
        reserved: j['reserved'] as int?,
        copies: j['copies'] as int?,
        holdingsUrl: j['holdingsUrl'] as String?,
        recordUrl: j['recordUrl'] as String?,
      );
}
