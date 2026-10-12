import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'record_models.dart';
import 'search_api.dart';
import 'search_models.dart';

final middlewareApiProvider = Provider<MiddlewareApi>((ref) => MiddlewareApi());

/// Per-record availability, keyed by the format's availabilityUrl.
/// Auto-disposed so counts for off-screen cards don't linger.
final availabilityProvider =
    FutureProvider.autoDispose.family<Availability, String>((ref, availabilityUrl) async {
  return ref.watch(middlewareApiProvider).availability(availabilityUrl);
});

/// Rich bib record, keyed by the format URL used to resolve it.
final recordDetailProvider =
    FutureProvider.autoDispose.family<RecordDetail, String>((ref, formatUrl) async {
  return ref.watch(middlewareApiProvider).record(formatUrl);
});

/// A related-titles panel (similar / author / series / versions), keyed by URL.
final relatedProvider =
    FutureProvider.autoDispose.family<RelatedList, String>((ref, panelUrl) async {
  return ref.watch(middlewareApiProvider).related(panelUrl);
});
