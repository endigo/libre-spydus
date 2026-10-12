import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../search/search_providers.dart';
import 'account_models.dart';
import 'account_providers.dart';
import 'login_screen.dart';

const _teal = Color(0xFF009CA6);

/// Account hub: sign-in prompt when signed out; loans / reservations / sign-out
/// when signed in.
class AccountScreen extends ConsumerWidget {
  const AccountScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final signedIn = ref.watch(isSignedInProvider);
    return Scaffold(
      appBar: AppBar(
        title: const Text('My account'),
        backgroundColor: _teal,
        foregroundColor: Colors.white,
      ),
      body: signedIn ? _AccountMenu(ref: ref) : const _SignedOut(),
    );
  }
}

class _SignedOut extends StatelessWidget {
  const _SignedOut();
  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.account_circle_outlined, size: 64, color: Colors.grey.shade400),
            const SizedBox(height: 16),
            const Text('Sign in to see your loans and reservations.', textAlign: TextAlign.center),
            const SizedBox(height: 20),
            FilledButton.icon(
              style: FilledButton.styleFrom(backgroundColor: _teal),
              onPressed: () => Navigator.of(context).push(
                MaterialPageRoute(builder: (_) => const LoginScreen()),
              ),
              icon: const Icon(Icons.login),
              label: const Text('Sign in'),
            ),
          ],
        ),
      ),
    );
  }
}

class _AccountMenu extends StatelessWidget {
  const _AccountMenu({required this.ref});
  final WidgetRef ref;

  @override
  Widget build(BuildContext context) {
    return ListView(
      children: [
        ListTile(
          leading: const Icon(Icons.menu_book, color: _teal),
          title: const Text('My loans'),
          subtitle: const Text('Items you have on loan'),
          trailing: const Icon(Icons.chevron_right),
          onTap: () => Navigator.of(context).push(
            MaterialPageRoute(builder: (_) => const MyLoansScreen()),
          ),
        ),
        const Divider(height: 1),
        ListTile(
          leading: const Icon(Icons.bookmark, color: _teal),
          title: const Text('My reservations'),
          subtitle: const Text('Holds ready for pickup and on the way'),
          trailing: const Icon(Icons.chevron_right),
          onTap: () => Navigator.of(context).push(
            MaterialPageRoute(builder: (_) => const MyReservationsScreen()),
          ),
        ),
        const Divider(height: 1),
        ListTile(
          leading: Icon(Icons.logout, color: Colors.red.shade400),
          title: const Text('Sign out'),
          onTap: () => ref.read(sessionProvider.notifier).signOut(),
        ),
      ],
    );
  }
}

// ---------------------------------------------------------------------------
// My loans
// ---------------------------------------------------------------------------

class MyLoansScreen extends ConsumerWidget {
  const MyLoansScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final async = ref.watch(loansProvider);
    return Scaffold(
      appBar: AppBar(title: const Text('My loans'), backgroundColor: _teal, foregroundColor: Colors.white),
      body: RefreshIndicator(
        onRefresh: () async => ref.invalidate(loansProvider),
        child: async.when(
          loading: () => const _Loading(),
          error: (e, _) => _ErrorView(onRetry: () => ref.invalidate(loansProvider)),
          data: (list) {
            if (list.items.isEmpty) {
              return const _Empty(icon: Icons.menu_book_outlined, text: 'You have no current loans.');
            }
            return ListView.separated(
              itemCount: list.items.length,
              separatorBuilder: (_, _) => const Divider(height: 1),
              itemBuilder: (_, i) {
                final it = list.items[i];
                return _PatronTile(
                  item: it,
                  trailing: it.due == null
                      ? null
                      : _Pill(label: 'Due ${it.due}', color: Colors.blueGrey),
                );
              },
            );
          },
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// My reservations
// ---------------------------------------------------------------------------

class MyReservationsScreen extends ConsumerWidget {
  const MyReservationsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final async = ref.watch(reservationsProvider);
    return Scaffold(
      appBar: AppBar(title: const Text('My reservations'), backgroundColor: _teal, foregroundColor: Colors.white),
      body: RefreshIndicator(
        onRefresh: () async => ref.invalidate(reservationsProvider),
        child: async.when(
          loading: () => const _Loading(),
          error: (e, _) => _ErrorView(onRetry: () => ref.invalidate(reservationsProvider)),
          data: (res) {
            if (res.ready.items.isEmpty && res.pending.items.isEmpty) {
              return const _Empty(icon: Icons.bookmark_border, text: 'You have no reservations.');
            }
            return ListView(
              children: [
                if (res.ready.items.isNotEmpty) ...[
                  const _SectionHeader('Ready for pickup', Icons.check_circle, Colors.green),
                  for (final it in res.ready.items) _ReservationTile(item: it),
                ],
                if (res.pending.items.isNotEmpty) ...[
                  const _SectionHeader('On the way', Icons.schedule, Colors.orange),
                  for (final it in res.pending.items) _ReservationTile(item: it),
                ],
              ],
            );
          },
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Edit-reservation sheet (pickup location + "not required after" date)
// ---------------------------------------------------------------------------

class _EditSheet extends ConsumerStatefulWidget {
  const _EditSheet({required this.item, required this.editUrl, required this.session});
  final PatronItem item;
  final String editUrl;
  final String session;

  @override
  ConsumerState<_EditSheet> createState() => _EditSheetState();
}

class _EditSheetState extends ConsumerState<_EditSheet> {
  EditOptions? _options; // null while loading
  Object? _error;
  String? _pickup; // selected pickup code
  DateTime? _expiry; // chosen "not required after", null = leave as-is
  bool _clearExpiry = false;
  String? _originalExpiry; // yyyy-mm-dd from the form
  bool _saving = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final o = await ref.read(middlewareApiProvider).editOptions(widget.editUrl, widget.session);
      if (!mounted) return;
      // Initialise every derived field together so the first render of the form
      // already has the pickup pre-selected (no dropdown/FutureBuilder race).
      setState(() {
        _options = o;
        _originalExpiry = o.notRequiredAfter;
        _expiry = _parse(o.notRequiredAfter);
        _pickup = _matchPickup(o);
      });
    } catch (e) {
      if (mounted) setState(() => _error = e);
    }
  }

  /// Pre-select the location the hold currently uses: the form's own selection,
  /// else the name parsed from the reservation status, else the first choice.
  String? _matchPickup(EditOptions o) {
    if (o.pickupSelected != null) return o.pickupSelected;
    final name = widget.item.pickupName?.toLowerCase();
    if (name != null) {
      for (final c in o.pickupChoices) {
        if (c.name.toLowerCase() == name) return c.code;
      }
    }
    return o.pickupChoices.isEmpty ? null : o.pickupChoices.first.code;
  }

  static DateTime? _parse(String? ymd) {
    if (ymd == null || ymd.isEmpty) return null;
    return DateTime.tryParse(ymd);
  }

  static String _fmt(DateTime d) =>
      '${d.year.toString().padLeft(4, '0')}-${d.month.toString().padLeft(2, '0')}-${d.day.toString().padLeft(2, '0')}';

  Future<void> _save() async {
    setState(() => _saving = true);
    final api = ref.read(middlewareApiProvider);

    // A blank date means "leave unchanged"; only send a date that differs from
    // the form's value, and only clear when a date was actually set.
    final newExpiry = _expiry == null ? null : _fmt(_expiry!);
    final sendExpiry = !_clearExpiry && newExpiry != null && newExpiry != (_originalExpiry ?? '');

    final messenger = ScaffoldMessenger.of(context);
    final result = await api.editReservation(
      widget.editUrl,
      widget.session,
      pickup: _pickup,
      notRequiredAfter: sendExpiry ? newExpiry : null,
      clearExpiry: _clearExpiry && _originalExpiry != null,
    );
    if (!mounted) return;
    messenger.showSnackBar(
      SnackBar(content: Text(result.message ?? (result.ok ? 'Reservation updated.' : 'Could not update.'))),
    );
    Navigator.of(context).pop(result.ok);
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: EdgeInsets.only(
        left: 20,
        right: 20,
        top: 4,
        bottom: MediaQuery.of(context).viewInsets.bottom + 20,
      ),
      child: _body(),
    );
  }

  Widget _body() {
    final o = _options;
    if (_error != null) {
      return const Padding(
        padding: EdgeInsets.all(24),
        child: Text('Could not load the reservation. Your session may have expired.'),
      );
    }
    if (o == null) {
      // A bare Center expands to the sheet's full height (isScrollControlled),
      // making the sheet open full-page then collapse. Keep it content-sized.
      return const Padding(
        padding: EdgeInsets.symmetric(vertical: 36),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            SizedBox(width: 28, height: 28, child: CircularProgressIndicator(strokeWidth: 3, color: _teal)),
          ],
        ),
      );
    }
    if (!o.canEdit) {
      return Padding(
        padding: const EdgeInsets.all(24),
        child: Text(o.message ?? 'This reservation can\'t be edited right now.'),
      );
    }
    final showClear = _expiry != null || (_originalExpiry != null && !_clearExpiry);
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('Edit reservation',
            style: Theme.of(context).textTheme.titleLarge?.copyWith(fontWeight: FontWeight.w700)),
        const SizedBox(height: 4),
        Text(widget.item.title ?? '',
            style: TextStyle(color: Colors.grey.shade700), maxLines: 2, overflow: TextOverflow.ellipsis),
        const SizedBox(height: 20),
        // Digital holds (ebooks) have no pickup branch — show the picker only
        // when the form actually offers locations.
        if (o.pickupChoices.isNotEmpty) ...[
          const Text('Pickup location', style: TextStyle(fontWeight: FontWeight.w600)),
          const SizedBox(height: 6),
          DropdownButtonFormField<String>(
            initialValue: _pickup,
            isExpanded: true,
            decoration: const InputDecoration(border: OutlineInputBorder(), isDense: true),
            items: [
              for (final c in o.pickupChoices) DropdownMenuItem(value: c.code, child: Text(c.name)),
            ],
            onChanged: _saving ? null : (v) => setState(() => _pickup = v),
          ),
          const SizedBox(height: 20),
        ],
        const Text('Not required after', style: TextStyle(fontWeight: FontWeight.w600)),
        const SizedBox(height: 2),
        Text('Cancel the hold automatically after this date (optional).',
            style: TextStyle(fontSize: 12, color: Colors.grey.shade600)),
        const SizedBox(height: 6),
        Row(
          children: [
            Expanded(
              child: OutlinedButton.icon(
                icon: const Icon(Icons.event_outlined, size: 18),
                label: Text(_clearExpiry
                    ? 'No expiry'
                    : (_expiry == null ? 'Set a date' : _fmt(_expiry!))),
                onPressed: _saving
                    ? null
                    : () async {
                        final now = DateTime.now();
                        final picked = await showDatePicker(
                          context: context,
                          initialDate: _expiry ?? now.add(const Duration(days: 30)),
                          firstDate: now,
                          lastDate: now.add(const Duration(days: 730)),
                        );
                        if (picked != null) {
                          setState(() {
                            _expiry = picked;
                            _clearExpiry = false;
                          });
                        }
                      },
              ),
            ),
            if (showClear)
              IconButton(
                tooltip: 'Clear',
                icon: const Icon(Icons.clear),
                onPressed: _saving
                    ? null
                    : () => setState(() {
                          _expiry = null;
                          _clearExpiry = true;
                        }),
              ),
          ],
        ),
        const SizedBox(height: 24),
        SizedBox(
          width: double.infinity,
          child: FilledButton(
            style: FilledButton.styleFrom(backgroundColor: _teal),
            onPressed: _saving ? null : _save,
            child: _saving
                ? const SizedBox(
                    height: 20, width: 20,
                    child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white))
                : const Text('Save changes'),
          ),
        ),
      ],
    );
  }
}

// ---------------------------------------------------------------------------
// Shared pieces
// ---------------------------------------------------------------------------

class _PatronTile extends StatelessWidget {
  const _PatronTile({required this.item, this.trailing, this.subtitleOverride, this.menu});
  final PatronItem item;
  final Widget? trailing;
  final String? subtitleOverride;

  /// Optional action control pinned to the trailing edge of the row.
  final Widget? menu;

  @override
  Widget build(BuildContext context) {
    final meta = [
      if (item.author != null) item.author!,
      if (item.year != null) item.year!,
    ].join(' · ');
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 12),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _Cover(url: item.coverUrl),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(item.title ?? 'Untitled',
                    style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 14, height: 1.25)),
                if (meta.isNotEmpty) ...[
                  const SizedBox(height: 3),
                  Text(meta, style: TextStyle(fontSize: 12, color: Colors.grey.shade700)),
                ],
                if (subtitleOverride != null) ...[
                  const SizedBox(height: 6),
                  Text(subtitleOverride!, style: TextStyle(fontSize: 12, color: Colors.grey.shade800)),
                ],
                if (trailing != null) ...[
                  const SizedBox(height: 8),
                  trailing!,
                ],
              ],
            ),
          ),
          ?menu,
        ],
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Reservation tile with Cancel / Edit actions
// ---------------------------------------------------------------------------

class _ReservationTile extends ConsumerWidget {
  const _ReservationTile({required this.item});
  final PatronItem item;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final canCancel = item.cancelUrl != null;
    final canEdit = item.editUrl != null;
    final menu = (!canCancel && !canEdit)
        ? null
        : PopupMenuButton<String>(
            icon: const Icon(Icons.more_vert),
            tooltip: 'Reservation actions',
            onSelected: (v) {
              if (v == 'cancel') _confirmCancel(context, ref);
              if (v == 'edit') _openEdit(context, ref);
            },
            itemBuilder: (_) => [
              if (canEdit)
                const PopupMenuItem(
                  value: 'edit',
                  child: ListTile(
                    dense: true,
                    contentPadding: EdgeInsets.zero,
                    leading: Icon(Icons.edit_outlined),
                    title: Text('Edit'),
                  ),
                ),
              if (canCancel)
                PopupMenuItem(
                  value: 'cancel',
                  child: ListTile(
                    dense: true,
                    contentPadding: EdgeInsets.zero,
                    leading: Icon(Icons.cancel_outlined, color: Colors.red.shade400),
                    title: Text('Cancel', style: TextStyle(color: Colors.red.shade400)),
                  ),
                ),
            ],
          );
    return _PatronTile(item: item, subtitleOverride: item.status, menu: menu);
  }

  Future<void> _confirmCancel(BuildContext context, WidgetRef ref) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Cancel reservation?'),
        content: Text(
          'This will cancel your hold on "${item.title ?? 'this title'}". '
          'You\'ll lose your place in the queue.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('Keep it'),
          ),
          FilledButton(
            style: FilledButton.styleFrom(backgroundColor: Colors.red.shade400),
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('Cancel hold'),
          ),
        ],
      ),
    );
    if (ok != true || !context.mounted) return;

    final session = ref.read(sessionProvider);
    final url = item.cancelUrl;
    if (session == null || url == null) return;

    final messenger = ScaffoldMessenger.of(context);
    messenger.showSnackBar(const SnackBar(content: Text('Cancelling…')));
    final result = await ref.read(middlewareApiProvider).cancelReservation(url, session);
    messenger.hideCurrentSnackBar();
    messenger.showSnackBar(
      SnackBar(content: Text(result.message ?? (result.ok ? 'Reservation cancelled.' : 'Could not cancel.'))),
    );
    if (result.ok) ref.invalidate(reservationsProvider);
  }

  Future<void> _openEdit(BuildContext context, WidgetRef ref) async {
    final session = ref.read(sessionProvider);
    final url = item.editUrl;
    if (session == null || url == null) return;
    final changed = await showModalBottomSheet<bool>(
      context: context,
      isScrollControlled: true,
      showDragHandle: true,
      builder: (_) => _EditSheet(item: item, editUrl: url, session: session),
    );
    if (changed == true) ref.invalidate(reservationsProvider);
  }
}

class _Pill extends StatelessWidget {
  const _Pill({required this.label, required this.color});
  final String label;
  final Color color;
  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(color: color.withValues(alpha: 0.12), borderRadius: BorderRadius.circular(12)),
      child: Text(label, style: TextStyle(fontSize: 12, color: color, fontWeight: FontWeight.w600)),
    );
  }
}

class _SectionHeader extends StatelessWidget {
  const _SectionHeader(this.title, this.icon, this.color);
  final String title;
  final IconData icon;
  final Color color;
  @override
  Widget build(BuildContext context) {
    return Container(
      color: color.withValues(alpha: 0.06),
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
      child: Row(
        children: [
          Icon(icon, size: 18, color: color),
          const SizedBox(width: 8),
          Text(title, style: TextStyle(fontWeight: FontWeight.w700, color: color)),
        ],
      ),
    );
  }
}

class _Cover extends StatelessWidget {
  const _Cover({required this.url});
  final String? url;
  @override
  Widget build(BuildContext context) {
    const w = 48.0, h = 68.0;
    final ph = Container(
      width: w,
      height: h,
      decoration: BoxDecoration(color: Colors.grey.shade200, borderRadius: BorderRadius.circular(4)),
      child: Icon(Icons.menu_book, size: 20, color: Colors.grey.shade400),
    );
    if (url == null) return ph;
    return ClipRRect(
      borderRadius: BorderRadius.circular(4),
      child: Image.network(url!, width: w, height: h, fit: BoxFit.cover, errorBuilder: (_, _, _) => ph),
    );
  }
}

class _Loading extends StatelessWidget {
  const _Loading();
  @override
  Widget build(BuildContext context) =>
      const Center(child: Padding(padding: EdgeInsets.all(32), child: CircularProgressIndicator(color: _teal)));
}

class _Empty extends StatelessWidget {
  const _Empty({required this.icon, required this.text});
  final IconData icon;
  final String text;
  @override
  Widget build(BuildContext context) {
    // ListView so RefreshIndicator still works on an empty list.
    return ListView(
      children: [
        const SizedBox(height: 120),
        Icon(icon, size: 56, color: Colors.grey.shade300),
        const SizedBox(height: 12),
        Text(text, textAlign: TextAlign.center, style: TextStyle(color: Colors.grey.shade600)),
      ],
    );
  }
}

class _ErrorView extends StatelessWidget {
  const _ErrorView({required this.onRetry});
  final VoidCallback onRetry;
  @override
  Widget build(BuildContext context) {
    return ListView(
      children: [
        const SizedBox(height: 100),
        Icon(Icons.cloud_off, size: 48, color: Colors.grey.shade400),
        const SizedBox(height: 12),
        const Center(child: Text('Could not load. Your session may have expired.')),
        const SizedBox(height: 16),
        Center(
          child: OutlinedButton.icon(onPressed: onRetry, icon: const Icon(Icons.refresh), label: const Text('Retry')),
        ),
      ],
    );
  }
}
