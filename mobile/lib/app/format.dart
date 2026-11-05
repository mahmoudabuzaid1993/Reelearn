import 'package:flutter/widgets.dart';
import 'package:intl/intl.dart';

/// `2026-11-02` -> `2 Nov 2026` in the current locale. Unparseable input is
/// returned as is so the UI never hides a server value.
String formatIsoDate(BuildContext context, String? iso) {
  if (iso == null || iso.isEmpty) return '';
  final d = DateTime.tryParse(iso);
  if (d == null) return iso;
  final locale = Localizations.localeOf(context).toLanguageTag();
  return DateFormat.yMMMd(locale).format(d);
}

String formatSeconds(int ms) => '${(ms / 1000).round()} s';
