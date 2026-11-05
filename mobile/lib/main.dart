import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'app/app.dart';
import 'core/offline/key_value_store.dart';
import 'core/providers.dart';
import 'features/player/data/telemetry.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await SystemChrome.setPreferredOrientations([DeviceOrientation.portraitUp]);
  final kv = await PrefsKeyValueStore.open();

  final container = ProviderContainer(
    overrides: [keyValueStoreProvider.overrideWithValue(kv)],
  );
  // Start the offline queue early so queued events replay on launch.
  await container.read(telemetryProvider).start();

  runApp(
    UncontrolledProviderScope(
      container: container,
      child: const ReelearnApp(),
    ),
  );
}
