import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../account/account_models.dart';
import '../account/account_providers.dart';
import '../account/login_screen.dart';
import 'record_models.dart';
import 'search_models.dart';
import 'search_providers.dart';

const _teal = Color(0xFF009CA6);

/// Book detail. Built either from a search [record] (shows live availability and
/// the reservation action) or, when a related title is tapped, directly from a
/// bib [recordUrl] (rich details only). The rich "Record details" block and the
/// Similar / By-author / In-series panels are loaded from the middleware.
class BookDetailScreen extends ConsumerWidget {
  const BookDetailScreen({super.key, required SearchRecord this.record})
      : recordUrl = null,
        fallbackTitle = null,
        fallbackCover = null;

  const BookDetailScreen.fromUrl({
    super.key,
    required String this.recordUrl,
    String? title,
    String? cover,
  })  : record = null,
        fallbackTitle = title,
        fallbackCover = cover;

  final SearchRecord? record;
  final String? recordUrl;
  final String? fallbackTitle;
  final String? fallbackCover;

  RecordFormat? get _primaryFormat {
    final r = record;
    if (r == null) return null;
    for (final f in r.formats) {
      if (f.code == 'BK' && f.availabilityUrl != null) return f;
    }
    for (final f in r.formats) {
      if (f.availabilityUrl != null) return f;
    }
    return r.formats.isEmpty ? null : r.formats.first;
  }

  /// The URL the middleware resolves the rich record from.
  String? get _sourceUrl => recordUrl ?? _primaryFormat?.availabilityUrl;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final sourceUrl = _sourceUrl;
    final detailAsync =
        sourceUrl == null ? null : ref.watch(recordDetailProvider(sourceUrl));
    final detail = detailAsync?.asData?.value;

    final title = record?.title ?? detail?.title ?? fallbackTitle ?? 'Untitled';
    final cover = record?.coverUrl ?? detail?.coverUrl ?? fallbackCover;
    final meta = [
      if (record?.year != null) record!.year!,
      if (record?.language != null) record!.language!,
    ].join(' · ');

    return Scaffold(
      appBar: AppBar(
        title: const Text('Details'),
        backgroundColor: _teal,
        foregroundColor: Colors.white,
      ),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(20, 20, 20, 32),
        children: [
          Center(child: _Cover(url: cover)),
          const SizedBox(height: 20),
          Text(title,
              style: const TextStyle(fontSize: 20, fontWeight: FontWeight.w700, height: 1.25)),
          if (record?.author != null) ...[
            const SizedBox(height: 8),
            Text(
              record!.authorRole != null ? '${record!.author} · ${record!.authorRole}' : record!.author!,
              style: TextStyle(fontSize: 15, color: Colors.grey.shade800),
            ),
          ],
          if (meta.isNotEmpty) ...[
            const SizedBox(height: 4),
            Text(meta, style: TextStyle(fontSize: 13, color: Colors.grey.shade600)),
          ],
          if (record != null && record!.formats.isNotEmpty) ...[
            const SizedBox(height: 14),
            Wrap(
              spacing: 8,
              runSpacing: 6,
              children: [for (final f in record!.formats) _FormatChip(label: f.label ?? f.code)],
            ),
          ],
          if (_primaryFormat != null) ...[
            const SizedBox(height: 20),
            _AvailabilityCard(format: _primaryFormat),
          ],

          // Summary: prefer the search record's, fall back to the rich record's.
          if ((record?.summary ?? detail?.summary) != null) ...[
            const SizedBox(height: 24),
            const _SectionTitle('Summary'),
            const SizedBox(height: 8),
            Text((record?.summary ?? detail?.summary)!,
                style: const TextStyle(fontSize: 14, height: 1.45)),
          ],

          // Rich record details + related panels.
          if (detailAsync != null)
            detailAsync.when(
              loading: () => const Padding(
                padding: EdgeInsets.only(top: 28),
                child: Center(child: CircularProgressIndicator(strokeWidth: 2)),
              ),
              error: (e, _) => Padding(
                padding: const EdgeInsets.only(top: 24),
                child: Text('Couldn\'t load full details.',
                    style: TextStyle(color: Colors.grey.shade600)),
              ),
              data: (d) => _DetailBody(detail: d),
            ),
        ],
      ),
    );
  }
}

/// Everything that comes from the rich /record response.
class _DetailBody extends StatelessWidget {
  const _DetailBody({required this.detail});
  final RecordDetail detail;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (detail.details.isNotEmpty) ...[
          const SizedBox(height: 24),
          const _SectionTitle('Record details'),
          const SizedBox(height: 8),
          for (final row in detail.details) _DetailRowView(row: row),
        ],
        _RelatedCarousel(title: 'In this series', url: detail.related.series),
        _RelatedCarousel(title: 'More by this author', url: detail.related.author),
        _RelatedCarousel(title: 'Other editions', url: detail.related.versions),
        _RelatedCarousel(title: 'Similar titles', url: detail.related.similar),
      ],
    );
  }
}

class _SectionTitle extends StatelessWidget {
  const _SectionTitle(this.text);
  final String text;
  @override
  Widget build(BuildContext context) =>
      Text(text, style: const TextStyle(fontWeight: FontWeight.w700, fontSize: 16));
}

/// One "Label: value | value" row, with linked values tinted teal.
class _DetailRowView extends StatelessWidget {
  const _DetailRowView({required this.row});
  final DetailRow row;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 104,
            child: Text(row.label,
                style: TextStyle(color: Colors.grey.shade600, fontSize: 13, height: 1.4)),
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                for (final v in row.values)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 2),
                    child: Text(
                      v.text,
                      style: TextStyle(
                        fontSize: 13,
                        height: 1.4,
                        color: v.url != null ? _teal : Colors.black87,
                      ),
                    ),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// A lazily-loaded horizontal strip of related titles. Renders nothing until it
/// has items (so empty/failed panels don't clutter the page).
class _RelatedCarousel extends ConsumerWidget {
  const _RelatedCarousel({required this.title, required this.url});
  final String title;
  final String? url;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final u = url;
    if (u == null) return const SizedBox.shrink();
    final async = ref.watch(relatedProvider(u));
    final items = async.asData?.value.items ?? const <RelatedItem>[];
    if (async.isLoading) {
      return Padding(
        padding: const EdgeInsets.only(top: 24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _SectionTitle(title),
            const SizedBox(height: 12),
            const SizedBox(
              height: 24,
              child: Center(child: SizedBox(width: 18, height: 18, child: CircularProgressIndicator(strokeWidth: 2))),
            ),
          ],
        ),
      );
    }
    if (items.isEmpty) return const SizedBox.shrink();
    return Padding(
      padding: const EdgeInsets.only(top: 24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _SectionTitle(title),
          const SizedBox(height: 12),
          SizedBox(
            height: 208,
            child: ListView.separated(
              scrollDirection: Axis.horizontal,
              itemCount: items.length,
              separatorBuilder: (_, _) => const SizedBox(width: 12),
              itemBuilder: (context, i) => _RelatedCard(item: items[i]),
            ),
          ),
        ],
      ),
    );
  }
}

class _RelatedCard extends StatelessWidget {
  const _RelatedCard({required this.item});
  final RelatedItem item;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 108,
      child: InkWell(
        borderRadius: BorderRadius.circular(6),
        onTap: item.bibUrl == null
            ? null
            : () => Navigator.of(context).push(MaterialPageRoute(
                  builder: (_) => BookDetailScreen.fromUrl(
                    recordUrl: item.bibUrl!,
                    title: item.title,
                    cover: item.coverUrl,
                  ),
                )),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _Cover(url: item.coverUrl, width: 108, height: 150),
            const SizedBox(height: 6),
            Text(
              item.title ?? '',
              maxLines: 3,
              overflow: TextOverflow.ellipsis,
              style: const TextStyle(fontSize: 12, height: 1.25, fontWeight: FontWeight.w500),
            ),
          ],
        ),
      ),
    );
  }
}

/// Spydus e-formats: ebook, e-audio, e-video, e-magazine, e-music. These are
/// borrowed online via an external service — no branch copies, no OPAC hold.
const _digitalFormats = {'EBK', 'EAUD', 'EVID', 'EMAG', 'EMUS', 'EJNL'};

bool _isDigitalFormat(String? code) => code != null && _digitalFormats.contains(code);

class _AvailabilityCard extends ConsumerWidget {
  const _AvailabilityCard({required this.format});
  final RecordFormat? format;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final url = format?.availabilityUrl;
    if (url == null) return const SizedBox.shrink();

    final async = ref.watch(availabilityProvider(url));
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: _teal.withValues(alpha: 0.06),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: _teal.withValues(alpha: 0.2)),
      ),
      child: async.when(
        loading: () => const Row(
          children: [
            SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2)),
            SizedBox(width: 12),
            Text('Checking availability…'),
          ],
        ),
        error: (e, _) => Text('Availability unavailable', style: TextStyle(color: Colors.grey.shade600)),
        data: (a) {
          final available = (a.copies ?? 0) - (a.reserved ?? 0);
          final ok = available > 0;
          // Digital formats (ebook / e-audio / e-video …) have no branch copies
          // or pickup — they're borrowed online, so don't show branch wording.
          final isDigital = _isDigitalFormat(format?.code);
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Icon(
                    isDigital
                        ? Icons.cloud_done_outlined
                        : (ok ? Icons.check_circle : Icons.schedule),
                    size: 18,
                    color: isDigital ? _teal : (ok ? Colors.green.shade600 : Colors.orange.shade700),
                  ),
                  const SizedBox(width: 8),
                  Text(
                    isDigital
                        ? 'Available online'
                        : (a.hasCounts
                            ? (ok ? 'Available now' : 'All copies out')
                            : 'See branch availability'),
                    style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 15),
                  ),
                ],
              ),
              if (isDigital) ...[
                const SizedBox(height: 6),
                Text(
                  'A digital title — borrow it through the library\'s online service.',
                  style: TextStyle(fontSize: 12, color: Colors.grey.shade700),
                ),
              ],
              if (!isDigital && a.hasCounts) ...[
                const SizedBox(height: 10),
                Row(
                  children: [
                    _Stat(label: 'Copies', value: '${a.copies ?? 0}'),
                    const SizedBox(width: 24),
                    _Stat(label: 'Reserved', value: '${a.reserved ?? 0}'),
                    const SizedBox(width: 24),
                    _Stat(label: 'Available', value: '${available < 0 ? 0 : available}'),
                  ],
                ),
              ],
              if (a.publisher != null) ...[
                const SizedBox(height: 12),
                Text(a.publisher!, style: TextStyle(fontSize: 12, color: Colors.grey.shade700)),
              ],
              const SizedBox(height: 14),
              _ReserveButton(reserveUrl: format?.reserveUrl),
            ],
          );
        },
      ),
    );
  }
}

/// "Place reservation" action: prompts sign-in when signed out, otherwise opens
/// the confirmation sheet. Hidden when no reservation URL is available.
class _ReserveButton extends ConsumerWidget {
  const _ReserveButton({required this.reserveUrl});
  final String? reserveUrl;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final url = reserveUrl;
    if (url == null) return const SizedBox.shrink();
    final signedIn = ref.watch(isSignedInProvider);

    return SizedBox(
      width: double.infinity,
      child: FilledButton.icon(
        style: FilledButton.styleFrom(backgroundColor: _teal),
        icon: Icon(signedIn ? Icons.bookmark_add_outlined : Icons.login),
        label: Text(signedIn ? 'Place reservation' : 'Sign in to reserve'),
        onPressed: () async {
          if (!ref.read(isSignedInProvider)) {
            await Navigator.of(context).push(
              MaterialPageRoute(builder: (_) => const LoginScreen()),
            );
            if (!context.mounted || !ref.read(isSignedInProvider)) return;
          }
          if (!context.mounted) return;
          await showModalBottomSheet<void>(
            context: context,
            isScrollControlled: true,
            showDragHandle: true,
            builder: (_) => _ReserveSheet(reserveUrl: url),
          );
        },
      ),
    );
  }
}

/// Bottom sheet: loads the reservation request form, lets the user pick an
/// expiry, and places the hold on confirm.
class _ReserveSheet extends ConsumerStatefulWidget {
  const _ReserveSheet({required this.reserveUrl});
  final String reserveUrl;
  @override
  ConsumerState<_ReserveSheet> createState() => _ReserveSheetState();
}

class _ReserveSheetState extends ConsumerState<_ReserveSheet> {
  late Future<ReserveOptions> _options;
  String _xdays = '0';
  bool _placing = false;

  @override
  void initState() {
    super.initState();
    final session = ref.read(sessionProvider)!;
    _options = ref.read(middlewareApiProvider).reserveOptions(widget.reserveUrl, session);
  }

  Future<void> _confirm() async {
    final session = ref.read(sessionProvider);
    if (session == null) return;
    setState(() => _placing = true);
    final result = await ref
        .read(middlewareApiProvider)
        .placeReservation(widget.reserveUrl, session, xdays: _xdays);
    if (!mounted) return;
    setState(() => _placing = false);
    Navigator.of(context).pop();
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(result.message ?? (result.ok ? 'Reservation placed.' : 'Could not place reservation.')),
        backgroundColor: result.ok ? Colors.green.shade700 : Colors.red.shade700,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: EdgeInsets.only(
        left: 20, right: 20, top: 4,
        bottom: 20 + MediaQuery.of(context).viewInsets.bottom,
      ),
      child: FutureBuilder<ReserveOptions>(
        future: _options,
        builder: (context, snap) {
          if (snap.connectionState != ConnectionState.done) {
            return const Padding(
              padding: EdgeInsets.all(40),
              child: Center(child: CircularProgressIndicator(strokeWidth: 2)),
            );
          }
          final opts = snap.data;
          if (opts == null || !opts.canReserve) {
            return Padding(
              padding: const EdgeInsets.symmetric(vertical: 24),
              child: Text(opts?.message ?? 'This title can\'t be reserved right now.'),
            );
          }
          return Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text('Place reservation',
                  style: TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
              const SizedBox(height: 16),
              // No pickup location for digital holds — only show it when set.
              if (opts.pickupLocation != null) ...[
                _SheetRow(
                  icon: Icons.location_on_outlined,
                  label: 'Pickup location',
                  value: opts.pickupLocation!,
                ),
                const SizedBox(height: 12),
              ],
              if (opts.availableSummary != null) ...[
                _SheetRow(icon: Icons.inventory_2_outlined, label: 'Availability', value: opts.availableSummary!),
                const SizedBox(height: 12),
              ],
              const SizedBox(height: 4),
              Row(
                children: [
                  Icon(Icons.event_busy_outlined, size: 20, color: Colors.grey.shade700),
                  const SizedBox(width: 12),
                  const Text('Not required after', style: TextStyle(fontSize: 14)),
                  const Spacer(),
                  DropdownButton<String>(
                    value: _xdays,
                    underline: const SizedBox.shrink(),
                    items: [
                      for (final e in opts.expiryChoices)
                        DropdownMenuItem(value: e.days, child: Text(e.label)),
                    ],
                    onChanged: _placing ? null : (v) => setState(() => _xdays = v ?? '0'),
                  ),
                ],
              ),
              const SizedBox(height: 20),
              SizedBox(
                width: double.infinity,
                child: FilledButton(
                  style: FilledButton.styleFrom(
                    backgroundColor: _teal,
                    padding: const EdgeInsets.symmetric(vertical: 14),
                  ),
                  onPressed: _placing ? null : _confirm,
                  child: _placing
                      ? const SizedBox(width: 20, height: 20, child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white))
                      : const Text('Confirm reservation'),
                ),
              ),
            ],
          );
        },
      ),
    );
  }
}

class _SheetRow extends StatelessWidget {
  const _SheetRow({required this.icon, required this.label, required this.value});
  final IconData icon;
  final String label;
  final String value;
  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Icon(icon, size: 20, color: Colors.grey.shade700),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(label, style: TextStyle(fontSize: 12, color: Colors.grey.shade600)),
              const SizedBox(height: 2),
              Text(value, style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w500)),
            ],
          ),
        ),
      ],
    );
  }
}

class _Stat extends StatelessWidget {
  const _Stat({required this.label, required this.value});
  final String label;
  final String value;
  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(value, style: const TextStyle(fontSize: 20, fontWeight: FontWeight.w700, color: _teal)),
        Text(label, style: TextStyle(fontSize: 11, color: Colors.grey.shade600)),
      ],
    );
  }
}

class _FormatChip extends StatelessWidget {
  const _FormatChip({required this.label});
  final String label;
  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
      decoration: BoxDecoration(
        color: _teal.withValues(alpha: 0.10),
        borderRadius: BorderRadius.circular(14),
      ),
      child: Text(label, style: const TextStyle(fontSize: 12, color: _teal, fontWeight: FontWeight.w500)),
    );
  }
}

class _Cover extends StatelessWidget {
  const _Cover({required this.url, this.width = 150, this.height = 215});
  final String? url;
  final double width;
  final double height;

  @override
  Widget build(BuildContext context) {
    final placeholder = Container(
      width: width,
      height: height,
      decoration: BoxDecoration(color: Colors.grey.shade200, borderRadius: BorderRadius.circular(6)),
      child: Icon(Icons.menu_book, size: width * 0.32, color: Colors.grey.shade400),
    );
    if (url == null) return placeholder;
    return ClipRRect(
      borderRadius: BorderRadius.circular(6),
      child: Image.network(
        url!,
        width: width,
        height: height,
        fit: BoxFit.cover,
        errorBuilder: (_, _, _) => placeholder,
        loadingBuilder: (c, child, p) => p == null
            ? child
            : SizedBox(width: width, height: height, child: const Center(child: CircularProgressIndicator(strokeWidth: 2))),
      ),
    );
  }
}
