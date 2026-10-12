import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// Persisted authentication state, mirroring the original app's encrypted
/// `Account` row (access/refresh tokens + borrower identifiers from JWT claims).
class AuthTokens {
  final String accessToken;
  final String refreshToken;

  const AuthTokens({required this.accessToken, required this.refreshToken});

  bool get isValid => accessToken.isNotEmpty && refreshToken.isNotEmpty;
}

/// Abstraction so the HTTP/auth layer can be unit-tested without platform plugins.
abstract class TokenStore {
  Future<AuthTokens?> read();
  Future<void> write(AuthTokens tokens);
  Future<void> clear();
}

/// Secure, on-device implementation (Keystore/Keychain backed).
class SecureTokenStore implements TokenStore {
  SecureTokenStore([FlutterSecureStorage? storage])
      : _storage = storage ?? const FlutterSecureStorage();

  final FlutterSecureStorage _storage;
  static const _kAccess = 'spydus.accessToken';
  static const _kRefresh = 'spydus.refreshToken';

  @override
  Future<AuthTokens?> read() async {
    final access = await _storage.read(key: _kAccess);
    final refresh = await _storage.read(key: _kRefresh);
    if (access == null || refresh == null) return null;
    return AuthTokens(accessToken: access, refreshToken: refresh);
  }

  @override
  Future<void> write(AuthTokens tokens) async {
    await _storage.write(key: _kAccess, value: tokens.accessToken);
    await _storage.write(key: _kRefresh, value: tokens.refreshToken);
  }

  @override
  Future<void> clear() async {
    await _storage.delete(key: _kAccess);
    await _storage.delete(key: _kRefresh);
  }
}

/// In-memory store for tests and previews.
class InMemoryTokenStore implements TokenStore {
  AuthTokens? _tokens;
  InMemoryTokenStore([this._tokens]);

  @override
  Future<AuthTokens?> read() async => _tokens;
  @override
  Future<void> write(AuthTokens tokens) async => _tokens = tokens;
  @override
  Future<void> clear() async => _tokens = null;
}
