import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:reelearn_mobile/core/api/fixtures.dart';
import 'package:reelearn_mobile/core/api/models/models.dart';
import 'package:reelearn_mobile/core/auth/session.dart';
import 'package:reelearn_mobile/features/settings/presentation/settings_screen.dart';

import '../../helpers.dart';

void main() {
  testWidgets(
      'toggling captions and language PATCHes /me and updates the session',
      (tester) async {
    tallViewport(tester);
    final env = TestEnv(signedInAs: Fixtures.sara);
    await tester.pumpWidget(env.wrap(const SettingsScreen()));
    await tester.pumpAndSettle();

    expect(find.text('What Al Wadi Retail can see'), findsOneWidget);
    expect(find.text('· Which answer you chose'), findsOneWidget);

    await tester.tap(find.byKey(const Key('switch-captions')));
    await tester.pumpAndSettle();
    expect(env.api.patches.last.captionsOn, isFalse);
    expect(env.api.currentUser!.captionsOn, isFalse);

    await tester.tap(find.byKey(const Key('lang-en')));
    await tester.pumpAndSettle();
    expect(env.api.patches.last.uiLanguage, 'en');
    final me = containerOf(tester).read(meProvider)!;
    expect(me.uiLanguage, 'en');
    expect(me.captionsOn, isFalse);

    await tester.tap(find.byKey(const Key('channel-whatsapp')));
    await tester.pumpAndSettle();
    expect(env.api.patches.last.deliveryChannel, Channel.whatsapp);
  });

  testWidgets('pause for two weeks sets paused_until; resume clears it',
      (tester) async {
    tallViewport(tester);
    final env = TestEnv(signedInAs: Fixtures.sara);
    await tester.pumpWidget(env.wrap(const SettingsScreen()));
    await tester.pumpAndSettle();

    await tester.scrollUntilVisible(find.byKey(const Key('pause-tile')), 200);
    await tester.tap(find.byKey(const Key('pause-tile')));
    await tester.pumpAndSettle();
    expect(env.api.patches.last.pausedUntil, isNotNull);
    expect(find.byKey(const Key('paused-tile')), findsOneWidget);

    await tester.ensureVisible(find.byKey(const Key('resume')));
    await tester.tap(find.byKey(const Key('resume')));
    await tester.pumpAndSettle();
    expect(env.api.patches.last.toJson(), {'paused_until': null});
    expect(find.byKey(const Key('pause-tile')), findsOneWidget);
  });

  testWidgets('download everything stored about me queues an export',
      (tester) async {
    tallViewport(tester);
    final env = TestEnv(signedInAs: Fixtures.sara);
    await tester.pumpWidget(env.wrap(const SettingsScreen()));
    await tester.pumpAndSettle();
    await tester.scrollUntilVisible(
      find.byKey(const Key('download-data')),
      200,
    );
    await tester.tap(find.byKey(const Key('download-data')));
    await tester.pumpAndSettle();
    expect(find.textContaining('within a day'), findsOneWidget);
  });
}
