import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'api/dio_client.dart';
import 'api/fake_api.dart';
import 'api/http_api.dart';
import 'api/reelearn_api.dart';
import 'auth/session.dart';
import 'auth/token_store.dart';
import 'config.dart';
import 'offline/connectivity.dart';
import 'offline/key_value_store.dart';

final appConfigProvider =
    Provider<AppConfig>((_) => AppConfig.fromEnvironment());

final tokenStoreProvider = Provider<TokenStore>((_) => SecureTokenStore());

/// Opened asynchronously in `main` and overridden there; tests override with
/// [MemoryKeyValueStore].
final keyValueStoreProvider = Provider<KeyValueStore>(
  (_) => throw StateError('keyValueStoreProvider must be overridden'),
);

final connectivityProvider =
    Provider<ConnectivityWatcher>((_) => PluginConnectivityWatcher());

/// The one API the app uses. `API=fake` swaps in the in-memory server.
final apiProvider = Provider<ReelearnApi>((ref) {
  final config = ref.watch(appConfigProvider);
  if (config.isFake) return FakeApi();
  final dio = buildDio(
    baseUrl: config.baseUrl,
    tokens: ref.watch(tokenStoreProvider),
    onSessionLost: () => ref.read(sessionProvider.notifier).sessionLost(),
  );
  return HttpApi(dio);
});
