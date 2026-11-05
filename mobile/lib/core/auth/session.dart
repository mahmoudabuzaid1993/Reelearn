import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../api/models/models.dart';
import '../errors/app_error.dart';
import '../offline/key_value_store.dart';
import '../providers.dart';
import 'token_store.dart';

sealed class SessionState {
  const SessionState();
}

class SessionLoading extends SessionState {
  const SessionLoading();
}

class SignedOut extends SessionState {
  const SignedOut();
}

class SignedIn extends SessionState {
  const SignedIn(this.me, {required this.needsWelcome});
  final Me me;

  /// First open on this device: show `/welcome` once.
  final bool needsWelcome;
}

String welcomeSeenKey(String userId) => 'reelearn.welcome_seen.$userId';

/// Owns the signed-in user. Tokens live in [TokenStore]; the user profile is
/// cached there too so the app opens straight to Today when offline.
class SessionNotifier extends Notifier<SessionState> {
  @override
  SessionState build() {
    unawaited(_restore());
    return const SessionLoading();
  }

  TokenStore get _tokens => ref.read(tokenStoreProvider);
  KeyValueStore get _kv => ref.read(keyValueStoreProvider);

  Future<void> _restore() async {
    final stored = await _tokens.read();
    final user = stored?.user;
    if (stored == null || user == null) {
      state = const SignedOut();
      return;
    }
    state = SignedIn(user, needsWelcome: await _needsWelcome(user));
    // Refresh the profile in the background; a failure keeps the cached one.
    try {
      final fresh = await ref.read(apiProvider).getMe();
      await _tokens.write(stored.copyWith(user: fresh));
      if (state is SignedIn) {
        state = SignedIn(fresh, needsWelcome: await _needsWelcome(fresh));
      }
    } on SessionLostError {
      // onSessionLost already moved us to SignedOut.
    } on AppError {
      // Offline or server trouble: keep the cached profile.
    }
  }

  Future<bool> _needsWelcome(Me me) async {
    if (!me.isFirstOpen) return false;
    final seen = await _kv.getBool(welcomeSeenKey(me.id)) ?? false;
    return !seen;
  }

  Future<void> signedIn(TokenPair pair) async {
    await _tokens.write(StoredSession.fromPair(pair));
    state = SignedIn(pair.user, needsWelcome: await _needsWelcome(pair.user));
  }

  Future<void> updateMe(Me me) async {
    final stored = await _tokens.read();
    if (stored != null) await _tokens.write(stored.copyWith(user: me));
    final current = state;
    if (current is SignedIn) {
      state = SignedIn(me, needsWelcome: current.needsWelcome);
    }
  }

  Future<void> welcomeDone() async {
    final current = state;
    if (current is! SignedIn) return;
    await _kv.setBool(welcomeSeenKey(current.me.id), value: true);
    state = SignedIn(current.me, needsWelcome: false);
  }

  Future<void> signOut() async {
    try {
      await ref.read(apiProvider).logout();
    } on AppError {
      // Best effort; the local session goes regardless.
    }
    await _tokens.clear();
    state = const SignedOut();
  }

  /// Called by the auth interceptor when a refresh fails.
  Future<void> sessionLost() async {
    await _tokens.clear();
    state = const SignedOut();
  }
}

final sessionProvider = NotifierProvider<SessionNotifier, SessionState>(
  SessionNotifier.new,
);

/// Convenience: the signed-in user or null.
final meProvider = Provider<Me?>((ref) {
  final s = ref.watch(sessionProvider);
  return s is SignedIn ? s.me : null;
});
