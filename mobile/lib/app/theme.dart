import 'package:flutter/cupertino.dart' show CupertinoPageTransitionsBuilder;
import 'package:flutter/material.dart';
import 'package:google_fonts/google_fonts.dart';

import 'locale.dart';

/// Brand tokens. No gradients, no emoji, mint only for feedback.
abstract final class Brand {
  static const ink = Color(0xFF0D2B3E);
  static const coral = Color(0xFFFF6257);
  static const mint = Color(0xFF2EC4B6);
  static const paper = Color(0xFFF6F7F5);
  static const night = Color(0xFF081822);
  static const slate = Color(0xFF5B6B78);
  static const border = Color(0xFFDDE2E0);
  static const white = Color(0xFFFFFFFF);

  static const radius = 14.0;
  static const target = 44.0;
}

/// Builds the app theme for a locale. `useGoogleFonts: false` keeps widget
/// tests hermetic (no font fetch).
ThemeData buildTheme({required Locale locale, bool useGoogleFonts = true}) {
  final isArabicScript = isRtl(locale);
  final base = ThemeData(
    useMaterial3: true,
    brightness: Brightness.light,
    colorScheme: const ColorScheme(
      brightness: Brightness.light,
      primary: Brand.coral,
      onPrimary: Brand.white,
      secondary: Brand.mint,
      onSecondary: Brand.night,
      error: Brand.slate,
      onError: Brand.white,
      surface: Brand.paper,
      onSurface: Brand.ink,
      outline: Brand.border,
      surfaceContainerHighest: Brand.white,
    ),
    scaffoldBackgroundColor: Brand.paper,
  );

  TextTheme text = base.textTheme.apply(
    bodyColor: Brand.ink,
    displayColor: Brand.ink,
  );
  if (useGoogleFonts) {
    text = isArabicScript
        ? GoogleFonts.ibmPlexSansArabicTextTheme(text)
        : GoogleFonts.readexProTextTheme(text);
  }

  const shape = RoundedRectangleBorder(
    borderRadius: BorderRadius.all(Radius.circular(Brand.radius)),
  );

  return base.copyWith(
    textTheme: text,
    appBarTheme: AppBarTheme(
      backgroundColor: Brand.paper,
      foregroundColor: Brand.ink,
      elevation: 0,
      scrolledUnderElevation: 0,
      centerTitle: false,
      titleTextStyle: text.titleLarge?.copyWith(
        fontWeight: FontWeight.w600,
        color: Brand.ink,
      ),
    ),
    cardTheme: const CardThemeData(
      color: Brand.white,
      elevation: 0,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.all(Radius.circular(Brand.radius)),
        side: BorderSide(color: Brand.border),
      ),
      margin: EdgeInsets.zero,
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        backgroundColor: Brand.coral,
        foregroundColor: Brand.white,
        minimumSize: const Size.fromHeight(52),
        shape: shape,
        textStyle: text.titleMedium?.copyWith(fontWeight: FontWeight.w600),
      ),
    ),
    elevatedButtonTheme: ElevatedButtonThemeData(
      style: ElevatedButton.styleFrom(
        backgroundColor: Brand.coral,
        foregroundColor: Brand.white,
        elevation: 0,
        minimumSize: const Size.fromHeight(52),
        shape: shape,
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        foregroundColor: Brand.ink,
        minimumSize: const Size(Brand.target, 48),
        side: const BorderSide(color: Brand.border),
        shape: shape,
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(
        foregroundColor: Brand.ink,
        minimumSize: const Size(Brand.target, Brand.target),
      ),
    ),
    inputDecorationTheme: const InputDecorationTheme(
      filled: true,
      fillColor: Brand.white,
      border: OutlineInputBorder(
        borderRadius: BorderRadius.all(Radius.circular(Brand.radius)),
        borderSide: BorderSide(color: Brand.border),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.all(Radius.circular(Brand.radius)),
        borderSide: BorderSide(color: Brand.border),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.all(Radius.circular(Brand.radius)),
        borderSide: BorderSide(color: Brand.ink, width: 1.5),
      ),
      contentPadding: EdgeInsets.symmetric(horizontal: 16, vertical: 14),
    ),
    chipTheme: ChipThemeData(
      backgroundColor: Brand.white,
      selectedColor: Brand.ink,
      side: const BorderSide(color: Brand.border),
      labelStyle: text.labelLarge?.copyWith(color: Brand.ink),
      secondaryLabelStyle: text.labelLarge?.copyWith(color: Brand.white),
      shape: const StadiumBorder(),
      showCheckmark: false,
    ),
    navigationBarTheme: NavigationBarThemeData(
      backgroundColor: Brand.white,
      indicatorColor: Brand.paper,
      height: 64,
      labelTextStyle: WidgetStatePropertyAll(text.labelMedium),
      iconTheme: const WidgetStatePropertyAll(IconThemeData(color: Brand.ink)),
    ),
    dividerTheme: const DividerThemeData(color: Brand.border, space: 1),
    switchTheme: SwitchThemeData(
      thumbColor: const WidgetStatePropertyAll(Brand.white),
      trackColor: WidgetStateProperty.resolveWith(
        (s) => s.contains(WidgetState.selected) ? Brand.ink : Brand.border,
      ),
      trackOutlineColor: const WidgetStatePropertyAll(Colors.transparent),
    ),
    snackBarTheme: SnackBarThemeData(
      backgroundColor: Brand.ink,
      contentTextStyle: text.bodyMedium?.copyWith(color: Brand.white),
      behavior: SnackBarBehavior.floating,
      shape: shape,
    ),
    pageTransitionsTheme: const PageTransitionsTheme(
      builders: {
        TargetPlatform.android: FadeUpwardsPageTransitionsBuilder(),
        TargetPlatform.iOS: CupertinoPageTransitionsBuilder(),
      },
    ),
  );
}
