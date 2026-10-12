/// Dart mirrors of the middleware's authenticated-account JSON.
library;

class PatronAction {
  const PatronAction({required this.label, required this.href});
  final String label;
  final String href;
  factory PatronAction.fromJson(Map<String, dynamic> j) =>
      PatronAction(label: j['label'] as String, href: j['href'] as String);
}

class PatronItem {
  const PatronItem({
    required this.irn,
    required this.bibIrn,
    required this.title,
    required this.bibUrl,
    required this.author,
    required this.year,
    required this.coverUrls,
    required this.isbn,
    required this.fields,
    required this.actions,
  });

  final String? irn;

  /// Bib-record IRN (matches a RecordDetail.bibIrn) — used to flag "reserved".
  final String? bibIrn;
  final String? title;
  final String? bibUrl;
  final String? author;
  final String? year;
  final List<String> coverUrls;
  final String? isbn;

  /// Columnar cells keyed by caption, e.g. {"Due": "27 Oct 2026"} /
  /// {"Status": "Please pickup from: RIVERTON LIBRARY by 17 Oct 2026"}.
  final Map<String, String> fields;
  final List<PatronAction> actions;

  String? get coverUrl => coverUrls.isEmpty ? null : coverUrls.first;
  String? get due => fields['Due'];
  String? get status => fields['Status'];

  /// The "Cancel reservation" action href, if this is a reservation row.
  String? get cancelUrl => _action('cancel');

  /// The "Edit reservation" action href, if this is a reservation row.
  String? get editUrl => _action('edit');

  String? _action(String needle) {
    for (final a in actions) {
      if (a.label.toLowerCase().contains(needle)) return a.href;
    }
    return null;
  }

  /// Pickup-location name parsed from the Status line, e.g. "RIVERTON LIBRARY"
  /// from "Requested pickup location: RIVERTON LIBRARY". Used to pre-select the
  /// current location in the edit sheet.
  String? get pickupName {
    final s = status;
    if (s == null) return null;
    final m = RegExp(r'(?:pickup (?:location|from)[:\s]+)([A-Z][A-Za-z0-9 ]+?)(?:\s+by\b|$)')
        .firstMatch(s);
    return m?.group(1)?.trim();
  }

  factory PatronItem.fromJson(Map<String, dynamic> j) => PatronItem(
        irn: j['irn'] as String?,
        bibIrn: j['bibIrn'] as String?,
        title: j['title'] as String?,
        bibUrl: j['bibUrl'] as String?,
        author: j['author'] as String?,
        year: j['year'] as String?,
        coverUrls:
            (j['coverUrls'] as List<dynamic>? ?? const []).map((e) => e as String).toList(),
        isbn: j['isbn'] as String?,
        fields: (j['fields'] as Map<String, dynamic>? ?? const {})
            .map((k, v) => MapEntry(k, v as String)),
        actions: (j['actions'] as List<dynamic>? ?? const [])
            .map((e) => PatronAction.fromJson(e as Map<String, dynamic>))
            .toList(),
      );
}

class PatronList {
  const PatronList({required this.total, required this.items});
  final int? total;
  final List<PatronItem> items;

  factory PatronList.fromJson(Map<String, dynamic> j) => PatronList(
        total: j['total'] as int?,
        items: (j['items'] as List<dynamic>? ?? const [])
            .map((e) => PatronItem.fromJson(e as Map<String, dynamic>))
            .toList(),
      );
}

class ReservationsResponse {
  const ReservationsResponse({required this.ready, required this.pending});
  final PatronList ready;
  final PatronList pending;

  factory ReservationsResponse.fromJson(Map<String, dynamic> j) => ReservationsResponse(
        ready: PatronList.fromJson(j['ready'] as Map<String, dynamic>? ?? const {}),
        pending: PatronList.fromJson(j['pending'] as Map<String, dynamic>? ?? const {}),
      );
}

class ExpiryChoice {
  const ExpiryChoice({required this.days, required this.label});
  final String days;
  final String label;
  factory ExpiryChoice.fromJson(Map<String, dynamic> j) =>
      ExpiryChoice(days: j['days'] as String? ?? '0', label: j['label'] as String? ?? '');
}

/// The reservation request form, shown before confirming a hold.
class ReserveOptions {
  const ReserveOptions({
    required this.canReserve,
    required this.pickupLocation,
    required this.availableSummary,
    required this.expiryChoices,
    required this.message,
  });

  final bool canReserve;
  final String? pickupLocation;
  final String? availableSummary;
  final List<ExpiryChoice> expiryChoices;
  final String? message;

  factory ReserveOptions.fromJson(Map<String, dynamic> j) => ReserveOptions(
        canReserve: j['canReserve'] as bool? ?? false,
        pickupLocation: j['pickupLocation'] as String?,
        availableSummary: j['availableSummary'] as String?,
        expiryChoices: (j['expiryChoices'] as List<dynamic>? ?? const [])
            .map((e) => ExpiryChoice.fromJson(e as Map<String, dynamic>))
            .toList(),
        message: j['message'] as String?,
      );
}

class ReserveResult {
  const ReserveResult({required this.ok, required this.message});
  final bool ok;
  final String? message;
  factory ReserveResult.fromJson(Map<String, dynamic> j) =>
      ReserveResult(ok: j['ok'] as bool? ?? false, message: j['message'] as String?);
}

/// A selectable pickup location on the edit-reservation form.
class PickupChoice {
  const PickupChoice({required this.code, required this.name});
  final String code;
  final String name;
  factory PickupChoice.fromJson(Map<String, dynamic> j) =>
      PickupChoice(code: j['code'] as String? ?? '', name: j['name'] as String? ?? '');
}

/// The editable fields of an existing reservation (pickup + optional dates).
class EditOptions {
  const EditOptions({
    required this.canEdit,
    required this.pickupChoices,
    required this.pickupSelected,
    required this.delayUntil,
    required this.notRequiredAfter,
    required this.message,
  });

  final bool canEdit;
  final List<PickupChoice> pickupChoices;
  final String? pickupSelected;
  final String? delayUntil;
  final String? notRequiredAfter;
  final String? message;

  factory EditOptions.fromJson(Map<String, dynamic> j) => EditOptions(
        canEdit: j['canEdit'] as bool? ?? false,
        pickupChoices: (j['pickupChoices'] as List<dynamic>? ?? const [])
            .map((e) => PickupChoice.fromJson(e as Map<String, dynamic>))
            .toList(),
        pickupSelected: j['pickupSelected'] as String?,
        delayUntil: j['delayUntil'] as String?,
        notRequiredAfter: j['notRequiredAfter'] as String?,
        message: j['message'] as String?,
      );
}
