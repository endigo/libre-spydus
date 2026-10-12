import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

import '../search/search_providers.dart';
import 'account_models.dart';

const _kSession = 'spydus_opac_session';
const _storage = FlutterSecureStorage();

/// Holds the current OPAC session token (null = signed out). Loaded from secure
/// storage on first build; updated on login/logout.
class SessionController extends Notifier<String?> {
  /// Debug hook: `--dart-define=INITIAL_SESSION=<token>` pre-seeds a session
  /// (used to inspect account screens on a headless simulator).
  static const _seed = String.fromEnvironment('INITIAL_SESSION');

  @override
  String? build() {
    if (_seed.isNotEmpty) return _seed;
    _restore();
    return null;
  }

  Future<void> _restore() async {
    try {
      final saved = await _storage.read(key: _kSession);
      if (saved != null && saved.isNotEmpty) state = saved;
    } catch (_) {
      // secure storage can throw on some platforms/first run; ignore.
    }
  }

  Future<void> signIn(String username, String password) async {
    final session = await ref.read(middlewareApiProvider).login(username, password);
    await _storage.write(key: _kSession, value: session);
    state = session;
  }

  Future<void> signOut() async {
    await _storage.delete(key: _kSession);
    state = null;
  }
}

final sessionProvider =
    NotifierProvider<SessionController, String?>(SessionController.new);

/// True once a session token is present.
final isSignedInProvider = Provider<bool>((ref) => ref.watch(sessionProvider) != null);

final loansProvider = FutureProvider.autoDispose<PatronList>((ref) async {
  final session = ref.watch(sessionProvider);
  if (session == null) throw StateError('Not signed in');
  return ref.watch(middlewareApiProvider).loans(session);
});

final reservationsProvider = FutureProvider.autoDispose<ReservationsResponse>((ref) async {
  final session = ref.watch(sessionProvider);
  if (session == null) throw StateError('Not signed in');
  return ref.watch(middlewareApiProvider).reservations(session);
});
