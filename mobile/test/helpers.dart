import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:reelearn_mobile/app/l10n/gen/app_localizations.dart';
import 'package:reelearn_mobile/app/locale.dart';
import 'package:reelearn_mobile/app/theme.dart';
import 'package:reelearn_mobile/core/api/fake_api.dart';
import 'package:reelearn_mobile/core/api/models/models.dart';
import 'package:reelearn_mobile/core/auth/token_store.dart';
import 'package:reelearn_mobile/core/offline/connectivity.dart';
import 'package:reelearn_mobile/core/offline/key_value_store.dart';
import 'package:reelearn_mobile/core/providers.dart';

/// A MaterialApp with the app's l10n and theme (no font fetching).
Widget wrapApp(Widget child, {Locale locale = const Locale('en')}) =>
    MaterialApp(
      locale: locale,
      supportedLocales: supportedUiLocales,
      localizationsDelegates: const [
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      theme: buildTheme(locale: locale, useGoogleFonts: false),
      home: child,
    );

/// Everything a screen needs: FakeApi, memory stores, fake connectivity.
class TestEnv {
  TestEnv({Me? signedInAs}) {
    if (signedInAs != null) {
      final pair = api.signInAs(signedInAs);
      tokens = MemoryTokenStore(StoredSession.fromPair(pair));
    } else {
      tokens = MemoryTokenStore();
    }
  }

  final api = FakeApi();
  final kv = MemoryKeyValueStore();
  final connectivity = FakeConnectivityWatcher();
  late final MemoryTokenStore tokens;

  List<Override> get overrides => [
        apiProvider.overrideWithValue(api),
        keyValueStoreProvider.overrideWithValue(kv),
        tokenStoreProvider.overrideWithValue(tokens),
        connectivityProvider.overrideWithValue(connectivity),
      ];

  Widget wrap(Widget child, {Locale locale = const Locale('en')}) =>
      ProviderScope(
        overrides: overrides,
        child: wrapApp(child, locale: locale),
      );
}

/// Reads the ProviderContainer behind a pumped [ProviderScope].
ProviderContainer containerOf(WidgetTester tester) =>
    ProviderScope.containerOf(tester.element(find.byType(MaterialApp)));

/// Lazily built lists only build what fits: give list screens a tall viewport.
void tallViewport(WidgetTester tester, {double height = 2400}) {
  tester.view.physicalSize = Size(800, height);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
}

/// True for error-red tones (coral counts: it is never allowed on a question
/// card). Mint, ink, slate, paper and white pass.
bool isReddish(Color c) {
  final r = (c.r * 255).round();
  final g = (c.g * 255).round();
  final b = (c.b * 255).round();
  return r > 160 && g < 120 && b < 120;
}

/// Walks the widget tree collecting every explicit colour it can see.
List<Color> visibleColors(WidgetTester tester) {
  final out = <Color>[];
  void add(Color? c) {
    if (c != null && c.a > 0) out.add(c);
  }

  for (final w in tester.allWidgets) {
    switch (w) {
      case Container(:final color, :final decoration):
        add(color);
        if (decoration is BoxDecoration) {
          add(decoration.color);
          final border = decoration.border;
          if (border is Border) add(border.top.color);
        }
      case DecoratedBox(:final decoration):
        if (decoration is BoxDecoration) add(decoration.color);
      case ColoredBox(:final color):
        add(color);
      case Material(:final color):
        add(color);
      case Text(:final style):
        add(style?.color);
      case Icon(:final color):
        add(color);
      default:
        break;
    }
  }
  return out;
}
