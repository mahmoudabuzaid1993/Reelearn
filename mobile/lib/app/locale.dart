import 'dart:ui';

/// Languages the app has strings for. Bengali (bn) is a learner language the
/// content supports; the UI falls back to English until its ARB exists.
const supportedUiLocales = [Locale('en'), Locale('ar'), Locale('ur')];

const learnerLanguages = ['ar', 'en', 'ur', 'bn'];

const _rtlLanguages = {'ar', 'ur', 'fa', 'he'};

bool isRtl(Locale locale) => _rtlLanguages.contains(locale.languageCode);

TextDirection textDirectionFor(Locale locale) =>
    isRtl(locale) ? TextDirection.rtl : TextDirection.ltr;

/// Maps the user's `ui_language` to a locale the app can render.
Locale resolveUiLocale(String? languageCode) {
  final code = (languageCode ?? 'en').toLowerCase();
  for (final l in supportedUiLocales) {
    if (l.languageCode == code) return l;
  }
  return const Locale('en');
}

/// `localeResolutionCallback` for MaterialApp: device locale, else English.
Locale resolveDeviceLocale(Locale? device, Iterable<Locale> supported) {
  if (device == null) return const Locale('en');
  for (final l in supported) {
    if (l.languageCode == device.languageCode) return l;
  }
  return const Locale('en');
}
