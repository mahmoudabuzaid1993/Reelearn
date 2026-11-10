import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:reelearn_mobile/core/api/fixtures.dart';
import 'package:reelearn_mobile/core/api/models/models.dart';
import 'package:reelearn_mobile/features/today/presentation/today_screen.dart';
import 'package:reelearn_mobile/features/topics/presentation/my_topics_screen.dart';

import '../../helpers.dart';

void main() {
  testWidgets(
      'follow and mute topics; required topics cannot be muted; personalisation switch',
      (tester) async {
    tallViewport(tester);
    final env = TestEnv(signedInAs: Fixtures.sara);
    await tester.pumpWidget(env.wrap(const MyTopicsScreen()));
    await tester.pumpAndSettle();
    expect(find.text('Customer service'), findsOneWidget);
    expect(find.text('Required'), findsNWidgets(2));
    expect(find.text('greeting'), findsOneWidget);

    final muteRequired = tester.widget<ChoiceChip>(
      find.byKey(const Key('mute-${Fixtures.topicStoreSafety}')),
    );
    expect(muteRequired.onSelected, isNull);

    await tester
        .tap(find.byKey(const Key('mute-${Fixtures.topicCustomerService}')));
    await tester.pumpAndSettle();
    final interests = await env.api.getMyTopics();
    expect(
      interests.topics
          .firstWhere((t) => t.topicId == Fixtures.topicCustomerService)
          .preference,
      TopicPreference.mute,
    );

    await tester.tap(find.byKey(const Key('personalized-switch')));
    await tester.pumpAndSettle();
    expect((await env.api.getMyTopics()).personalized, isFalse);
  });

  testWidgets(
      'a finished minute offers "Keep going" only when the company does',
      (tester) async {
    var kept = false;
    final done = Fixtures.plan.copyWith(status: PlanStatus.done);
    Widget body(DailyPlan plan, {VoidCallback? keep}) => wrapApp(
          Scaffold(
            body: TodayBody(
              plan: plan,
              name: 'Sara',
              onStart: () {},
              onOpenSaved: () {},
              onOpenDone: () {},
              onKeepGoing: keep,
            ),
          ),
        );
    await tester.pumpWidget(body(done));
    expect(find.byKey(const Key('today-keep-going')), findsNothing);
    await tester.pumpWidget(body(done, keep: () => kept = true));
    await tester.tap(find.byKey(const Key('today-keep-going')));
    expect(kept, isTrue);
  });
}
