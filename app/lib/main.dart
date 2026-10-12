import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'src/account/account_screen.dart';
import 'src/account/login_screen.dart';
import 'src/search/book_detail_screen.dart';
import 'src/search/search_screen.dart';

void main() => runApp(const ProviderScope(child: SpydusCloneApp()));

class SpydusCloneApp extends StatefulWidget {
  const SpydusCloneApp({super.key});

  /// Debug hook: --dart-define=INITIAL_ROUTE=account|login|loans|reservations
  /// opens a specific screen on launch (for a headless simulator).
  static const _initialRoute = String.fromEnvironment('INITIAL_ROUTE');

  /// Debug hook: `--dart-define=INITIAL_RECORD_URL=<bib url>` opens a book detail
  /// directly (for inspecting the rich record on a headless simulator).
  static const _initialRecordUrl = String.fromEnvironment('INITIAL_RECORD_URL');

  @override
  State<SpydusCloneApp> createState() => _SpydusCloneAppState();
}

class _SpydusCloneAppState extends State<SpydusCloneApp> {
  final _navKey = GlobalKey<NavigatorState>();

  @override
  void initState() {
    super.initState();
    // The deep-link hooks push their target ON TOP of the real home (Discover),
    // so the app bar gets a working back button instead of dead-ending.
    final target = _deepLinkTarget();
    if (target != null) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        _navKey.currentState?.push(MaterialPageRoute(builder: (_) => target));
      });
    }
  }

  Widget? _deepLinkTarget() {
    if (SpydusCloneApp._initialRecordUrl.isNotEmpty) {
      return BookDetailScreen.fromUrl(recordUrl: SpydusCloneApp._initialRecordUrl);
    }
    return switch (SpydusCloneApp._initialRoute) {
      'account' => const AccountScreen(),
      'login' => const LoginScreen(),
      'loans' => const MyLoansScreen(),
      'reservations' => const MyReservationsScreen(),
      _ => null,
    };
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Spydus Clone',
      navigatorKey: _navKey,
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xFF009CA6)),
        useMaterial3: true,
      ),
      home: const SearchScreen(),
    );
  }
}
