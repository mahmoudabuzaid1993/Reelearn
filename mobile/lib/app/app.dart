import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/auth/session.dart';
import 'l10n/gen/app_localizations.dart';
import 'locale.dart';
import 'router.dart';
import 'theme.dart';

/// `--dart-define=UI_LOCALE=en` pins the interface language (store screenshots,
/// demos, support sessions) regardless of the signed-in user's preference.
const _forcedUiLocale = String.fromEnvironment('UI_LOCALE');

/// The UI locale: the signed-in user's `ui_language`, else the device's.
final uiLocaleProvider = Provider<Locale?>((ref) {
  if (_forcedUiLocale.isNotEmpty) return resolveUiLocale(_forcedUiLocale);
  final me = ref.watch(meProvider);
  return me == null ? null : resolveUiLocale(me.uiLanguage);
});

class ReelearnApp extends ConsumerWidget {
  const ReelearnApp({super.key, this.useGoogleFonts = true});
  final bool useGoogleFonts;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final router = ref.watch(routerProvider);
    final locale = ref.watch(uiLocaleProvider);

    return MaterialApp.router(
      onGenerateTitle: (ctx) => AppLocalizations.of(ctx).appName,
      routerConfig: router,
      locale: locale,
      supportedLocales: supportedUiLocales,
      localeResolutionCallback: resolveDeviceLocale,
      localizationsDelegates: const [
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      theme: buildTheme(
        locale: locale ?? const Locale('en'),
        useGoogleFonts: useGoogleFonts,
      ),
      builder: (context, child) {
        final l = Localizations.localeOf(context);
        return Theme(
          data: buildTheme(locale: l, useGoogleFonts: useGoogleFonts),
          child: child ?? const SizedBox.shrink(),
        );
      },
    );
  }
}
