import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../account/account_screen.dart';
import '../config/app_config.dart';
import 'book_detail_screen.dart';
import 'search_api.dart';
import 'search_models.dart';
import 'search_providers.dart';

const _teal = Color(0xFF009CA6);

/// Discover / search screen — talks to the local middleware, which scrapes the
/// Canning OPAC and returns JSON.
class SearchScreen extends ConsumerStatefulWidget {
  const SearchScreen({super.key});

  @override
  ConsumerState<SearchScreen> createState() => _SearchScreenState();
}

class _SearchScreenState extends ConsumerState<SearchScreen> {
  final _controller = TextEditingController();
  String? _query;

  /// Debug hook: --dart-define=INITIAL_QUERY=... runs a search on launch
  /// (useful on a headless simulator with no keyboard).
  static const _initialQuery = String.fromEnvironment('INITIAL_QUERY');

  @override
  void initState() {
    super.initState();
    if (_initialQuery.isNotEmpty) {
      _controller.text = _initialQuery;
      _query = _initialQuery;
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _submit() {
    final q = _controller.text.trim();
    if (q.isEmpty) return;
    FocusScope.of(context).unfocus();
    setState(() => _query = q);
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Discover'),
        backgroundColor: _teal,
        foregroundColor: Colors.white,
        actions: [
          IconButton(
            tooltip: 'My account',
            icon: const Icon(Icons.account_circle),
            onPressed: () => Navigator.of(context).push(
              MaterialPageRoute(builder: (_) => const AccountScreen()),
            ),
          ),
        ],
      ),
      body: Column(
        children: [
          _SearchBar(controller: _controller, onSubmit: _submit),
          const Divider(height: 1),
          Expanded(
            child: _query == null
                ? const _Placeholder()
                // Key by query so a new search resets the pager state.
                : _Results(key: ValueKey(_query), query: _query!),
          ),
        ],
      ),
    );
  }
}

class _SearchBar extends StatelessWidget {
  const _SearchBar({required this.controller, required this.onSubmit});
  final TextEditingController controller;
  final VoidCallback onSubmit;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 12),
      child: Column(
        children: [
          TextField(
            controller: controller,
            autofocus: true,
            textInputAction: TextInputAction.search,
            onSubmitted: (_) => onSubmit(),
            decoration: InputDecoration(
              hintText: 'Search library catalogue',
              prefixIcon: const Icon(Icons.search),
              border: OutlineInputBorder(borderRadius: BorderRadius.circular(8)),
              isDense: true,
            ),
          ),
          const SizedBox(height: 10),
          SizedBox(
            width: double.infinity,
            child: FilledButton(
              onPressed: onSubmit,
              style: FilledButton.styleFrom(
                backgroundColor: _teal,
                padding: const EdgeInsets.symmetric(vertical: 14),
              ),
              child: const Text('Search'),
            ),
          ),
        ],
      ),
    );
  }
}

class _Placeholder extends StatelessWidget {
  const _Placeholder();
  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.menu_book_outlined, size: 48, color: Colors.grey.shade400),
            const SizedBox(height: 12),
            Text(
              'Search the ${AppConfig.libraryName} catalogue',
              textAlign: TextAlign.center,
              style: TextStyle(color: Colors.grey.shade600),
            ),
          ],
        ),
      ),
    );
  }
}

/// Paged results list with infinite scroll. Page 1 comes from `/search`;
/// subsequent pages are fetched from `/page?url=<nextPageUrl>` as the user
/// scrolls near the bottom.
class _Results extends ConsumerStatefulWidget {
  const _Results({super.key, required this.query});
  final String query;

  @override
  ConsumerState<_Results> createState() => _ResultsState();
}

class _ResultsState extends ConsumerState<_Results> {
  final _scroll = ScrollController();
  final List<SearchRecord> _records = [];
  String? _nextUrl;
  int? _total;
  bool _loading = true;
  bool _loadingMore = false;
  Object? _error;

  MiddlewareApi get _api => ref.read(middlewareApiProvider);

  @override
  void initState() {
    super.initState();
    _scroll.addListener(_onScroll);
    _loadFirst();
  }

  @override
  void dispose() {
    _scroll.dispose();
    super.dispose();
  }

  Future<void> _loadFirst() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final res = await _api.search(widget.query);
      if (!mounted) return;
      setState(() {
        _records
          ..clear()
          ..addAll(res.records);
        _nextUrl = res.nextPageUrl;
        _total = res.totalResults;
        _loading = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e;
        _loading = false;
      });
    }
  }

  Future<void> _loadMore() async {
    if (_loadingMore || _nextUrl == null) return;
    setState(() => _loadingMore = true);
    try {
      final res = await _api.page(_nextUrl!);
      if (!mounted) return;
      setState(() {
        _records.addAll(res.records);
        _nextUrl = res.nextPageUrl;
        _loadingMore = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() => _loadingMore = false);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Could not load more results: $e')),
      );
    }
  }

  void _onScroll() {
    if (!_scroll.hasClients) return;
    final pos = _scroll.position;
    if (pos.pixels >= pos.maxScrollExtent - 600) _loadMore();
  }

  @override
  Widget build(BuildContext context) {
    if (_loading) {
      return const Center(
        child: Padding(
          padding: EdgeInsets.all(32),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              CircularProgressIndicator(color: _teal),
              SizedBox(height: 16),
              Text(
                'Searching the catalogue…\nThe first search can take a few seconds.',
                textAlign: TextAlign.center,
              ),
            ],
          ),
        ),
      );
    }
    if (_error != null && _records.isEmpty) {
      return _ErrorView(query: widget.query, message: _error.toString(), onRetry: _loadFirst);
    }
    if (_records.isEmpty) {
      return Center(child: Text('No results for “${widget.query}”.'));
    }

    final hasMore = _nextUrl != null;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
          child: Text(
            _total != null ? 'Showing ${_records.length} of $_total' : '${_records.length} results',
            style: Theme.of(context).textTheme.labelLarge?.copyWith(color: Colors.grey.shade700),
          ),
        ),
        Expanded(
          child: ListView.separated(
            controller: _scroll,
            padding: const EdgeInsets.only(bottom: 24),
            itemCount: _records.length + (hasMore ? 1 : 0),
            separatorBuilder: (_, _) => const Divider(height: 1),
            itemBuilder: (_, i) {
              if (i >= _records.length) {
                return const Padding(
                  padding: EdgeInsets.symmetric(vertical: 20),
                  child: Center(
                    child: SizedBox(
                      width: 22,
                      height: 22,
                      child: CircularProgressIndicator(strokeWidth: 2, color: _teal),
                    ),
                  ),
                );
              }
              return _ResultCard(record: _records[i]);
            },
          ),
        ),
      ],
    );
  }
}

class _ResultCard extends ConsumerWidget {
  const _ResultCard({required this.record});
  final SearchRecord record;

  RecordFormat? get _primaryFormat {
    for (final f in record.formats) {
      if (f.code == 'BK' && f.availabilityUrl != null) return f;
    }
    for (final f in record.formats) {
      if (f.availabilityUrl != null) return f;
    }
    return record.formats.isEmpty ? null : record.formats.first;
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final subtitle = [
      if (record.author != null) record.author!,
      if (record.year != null) record.year!,
    ].join(' · ');

    return InkWell(
      onTap: () => Navigator.of(context).push(
        MaterialPageRoute(builder: (_) => BookDetailScreen(record: record)),
      ),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 14, 16, 14),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _Cover(url: record.coverUrl),
            const SizedBox(width: 14),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    record.title ?? 'Untitled',
                    style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 15, height: 1.25),
                  ),
                  if (subtitle.isNotEmpty) ...[
                    const SizedBox(height: 4),
                    Text(subtitle, style: TextStyle(color: Colors.grey.shade700, fontSize: 13)),
                  ],
                  const SizedBox(height: 8),
                  Wrap(
                    spacing: 6,
                    runSpacing: 4,
                    children: [
                      for (final f in record.formats) _FormatChip(label: f.label ?? f.code),
                    ],
                  ),
                  const SizedBox(height: 8),
                  _AvailabilityLine(format: _primaryFormat),
                ],
              ),
            ),
            const Padding(
              padding: EdgeInsets.only(left: 4, top: 2),
              child: Icon(Icons.chevron_right, color: Colors.grey),
            ),
          ],
        ),
      ),
    );
  }
}

class _Cover extends StatelessWidget {
  const _Cover({required this.url});
  final String? url;

  @override
  Widget build(BuildContext context) {
    const w = 58.0, h = 84.0;
    final placeholder = Container(
      width: w,
      height: h,
      decoration: BoxDecoration(
        color: Colors.grey.shade200,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Icon(Icons.menu_book, color: Colors.grey.shade400),
    );
    if (url == null) return placeholder;
    return ClipRRect(
      borderRadius: BorderRadius.circular(4),
      child: Image.network(
        url!,
        width: w,
        height: h,
        fit: BoxFit.cover,
        errorBuilder: (_, _, _) => placeholder,
        loadingBuilder: (c, child, p) => p == null
            ? child
            : SizedBox(width: w, height: h, child: const Center(child: SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2)))),
      ),
    );
  }
}

class _FormatChip extends StatelessWidget {
  const _FormatChip({required this.label});
  final String label;
  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: _teal.withValues(alpha: 0.10),
        borderRadius: BorderRadius.circular(12),
      ),
      child: Text(label, style: const TextStyle(fontSize: 11, color: _teal, fontWeight: FontWeight.w500)),
    );
  }
}

/// Lazily fetches availability for the card's primary format.
class _AvailabilityLine extends ConsumerWidget {
  const _AvailabilityLine({required this.format});
  final RecordFormat? format;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final url = format?.availabilityUrl;
    if (url == null) return const SizedBox.shrink();

    final async = ref.watch(availabilityProvider(url));
    return async.when(
      loading: () => Text('Checking availability…',
          style: TextStyle(fontSize: 12, color: Colors.grey.shade500)),
      error: (_, _) => const SizedBox.shrink(),
      data: (a) {
        if (!a.hasCounts) {
          return Row(children: [
            Icon(Icons.info_outline, size: 14, color: Colors.grey.shade600),
            const SizedBox(width: 4),
            Text('View availability', style: TextStyle(fontSize: 12, color: Colors.grey.shade700)),
          ]);
        }
        final available = (a.copies ?? 0) - (a.reserved ?? 0);
        final ok = available > 0;
        return Row(
          children: [
            Icon(ok ? Icons.check_circle : Icons.schedule,
                size: 14, color: ok ? Colors.green.shade600 : Colors.orange.shade700),
            const SizedBox(width: 4),
            Text(
              'Copies: ${a.copies ?? 0}  ·  Reserved: ${a.reserved ?? 0}',
              style: TextStyle(fontSize: 12, color: Colors.grey.shade800, fontWeight: FontWeight.w500),
            ),
          ],
        );
      },
    );
  }
}

class _ErrorView extends StatelessWidget {
  const _ErrorView({required this.query, required this.message, required this.onRetry});
  final String query;
  final String message;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.cloud_off, size: 42, color: Colors.grey.shade400),
            const SizedBox(height: 12),
            const Text('Could not reach the search service.',
                textAlign: TextAlign.center, style: TextStyle(fontWeight: FontWeight.w600)),
            const SizedBox(height: 6),
            Text(message,
                textAlign: TextAlign.center,
                maxLines: 3,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(fontSize: 12, color: Colors.grey.shade600)),
            const SizedBox(height: 16),
            OutlinedButton.icon(
              onPressed: onRetry,
              icon: const Icon(Icons.refresh),
              label: const Text('Retry'),
            ),
          ],
        ),
      ),
    );
  }
}
