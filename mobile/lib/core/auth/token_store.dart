import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';

import '../api/models/models.dart';

class StoredSession {
  const StoredSession({
    required this.accessToken,
    required this.refreshToken,
    this.user,
  });

  factory StoredSession.fromPair(TokenPair pair) => StoredSession(
        accessToken: pair.accessToken,
        refreshToken: pair.refreshToken,
        user: pair.user,
      );

  final String accessToken;
  final String refreshToken;
  final Me? user;

  StoredSession copyWith({
    String? accessToken,
    String? refreshToken,
    Me? user,
  }) =>
      StoredSession(
        accessToken: accessToken ?? this.accessToken,
        refreshToken: refreshToken ?? this.refreshToken,
        user: user ?? this.user,
      );
}

/// Where tokens live. Secure storage in the app, memory in tests.
abstract class TokenStore {
  Future<StoredSession?> read();
  Future<void> write(StoredSession session);
  Future<void> clear();
}

class MemoryTokenStore implements TokenStore {
  MemoryTokenStore([this._session]);
  StoredSession? _session;

  @override
  Future<StoredSession?> read() async => _session;

  @override
  Future<void> write(StoredSession session) async => _session = session;

  @override
  Future<void> clear() async => _session = null;
}

class SecureTokenStore implements TokenStore {
  SecureTokenStore([FlutterSecureStorage? storage])
      : _storage = storage ??
            const FlutterSecureStorage(
              aOptions: AndroidOptions(encryptedSharedPreferences: true),
              iOptions: IOSOptions(
                accessibility: KeychainAccessibility.first_unlock_this_device,
              ),
            );

  static const _kAccess = 'reelearn.access';
  static const _kRefresh = 'reelearn.refresh';
  static const _kUser = 'reelearn.user';

  final FlutterSecureStorage _storage;
  StoredSession? _cache;

  @override
  Future<StoredSession?> read() async {
    if (_cache != null) return _cache;
    final access = await _storage.read(key: _kAccess);
    final refresh = await _storage.read(key: _kRefresh);
    if (access == null || refresh == null) return null;
    final userRaw = await _storage.read(key: _kUser);
    Me? user;
    if (userRaw != null) {
      final decoded = jsonDecode(userRaw);
      if (decoded is Map<String, dynamic>) user = Me.fromJson(decoded);
    }
    return _cache = StoredSession(
      accessToken: access,
      refreshToken: refresh,
      user: user,
    );
  }

  @override
  Future<void> write(StoredSession session) async {
    _cache = session;
    await _storage.write(key: _kAccess, value: session.accessToken);
    await _storage.write(key: _kRefresh, value: session.refreshToken);
    if (session.user != null) {
      await _storage.write(
        key: _kUser,
        value: jsonEncode(session.user!.toJson()),
      );
    }
  }

  @override
  Future<void> clear() async {
    _cache = null;
    await _storage.delete(key: _kAccess);
    await _storage.delete(key: _kRefresh);
    await _storage.delete(key: _kUser);
  }
}
