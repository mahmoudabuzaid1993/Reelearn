import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:reelearn_mobile/app/l10n/gen/app_localizations.dart';
import 'package:reelearn_mobile/app/locale.dart';
import 'package:reelearn_mobile/core/config.dart';

import '../helpers.dart';

void main() {
  test('RTL resolution: ar and ur mirror, en does not, bn falls back to en',
      () {
    expect(textDirectionFor(const Locale('ar')), TextDirection.rtl);
    expect(textDirectionFor(const Locale('ur')), TextDirection.rtl);
    expect(textDirectionFor(const Locale('en')), TextDirection.ltr);
    expect(resolveUiLocale('ar'), const Locale('ar'));
    expect(resolveUiLocale('ur'), const Locale('ur'));
    expect(resolveUiLocale('bn'), const Locale('en'));
    expect(resolveUiLocale(null), const Locale('en'));
    expect(
      resolveDeviceLocale(const Locale('ar', 'SA'), supportedUiLocales),
      const Locale('ar'),
    );
    expect(
      resolveDeviceLocale(const Locale('fr'), supportedUiLocales),
      const Locale('en'),
    );
  });

  test('every supported locale has the toast and no banned words', () async {
    for (final locale in supportedUiLocales) {
      final l = await AppLocalizations.delegate.load(locale);
      expect(l.answerFirstToast, isNotEmpty);
      expect(l.startMinute, isNotEmpty);
      for (final banned in ['scroll', 'feed', 'binge', 'addictive']) {
        expect(l.startMinute.toLowerCase(), isNot(contains(banned)));
        expect(l.swipeHint.toLowerCase(), isNot(contains(banned)));
        expect(l.todayTitle.toLowerCase(), isNot(contains(banned)));
      }
    }
    final ar = await AppLocalizations.delegate.load(const Locale('ar'));
    expect(ar.answerFirstToast, 'أجب أولًا، ثم اسحب للأعلى');
    expect(ar.newReelsCount(2), 'مقطعان جديدان');
  });

  testWidgets('an Arabic app renders right-to-left', (tester) async {
    late BuildContext captured;
    await tester.pumpWidget(
      wrapApp(
        Builder(
          builder: (context) {
            captured = context;
            return Text(AppLocalizations.of(context).todayTitle);
          },
        ),
        locale: const Locale('ar'),
      ),
    );
    expect(Directionality.of(captured), TextDirection.rtl);
    expect(find.text('اليوم'), findsOneWidget);
  });

  test('Android emulator base URL rewrites localhost to 10.0.2.2', () {
    expect(
      AppConfig.resolveBaseUrl(
        'http://localhost:8080/v1',
        TargetPlatform.android,
      ),
      'http://10.0.2.2:8080/v1',
    );
    expect(
      AppConfig.resolveBaseUrl('http://localhost:8080/v1', TargetPlatform.iOS),
      'http://localhost:8080/v1',
    );
  });
}
