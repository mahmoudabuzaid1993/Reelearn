import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:reelearn_mobile/core/api/fixtures.dart';
import 'package:reelearn_mobile/core/api/models/models.dart';
import 'package:reelearn_mobile/features/today/presentation/done_summary.dart';
import 'package:reelearn_mobile/features/today/presentation/today_screen.dart';

import '../../helpers.dart';

Widget today(DailyPlan plan, {VoidCallback? onStart}) => wrapApp(
      Scaffold(
        body: TodayBody(
          plan: plan,
          name: 'Sara',
          onStart: onStart ?? () {},
          onOpenSaved: () {},
          onOpenDone: () {},
        ),
      ),
    );

void main() {
  testWidgets('open plan: summary, streak chip, yesterday line, start button',
      (tester) async {
    var started = false;
    await tester
        .pumpWidget(today(Fixtures.plan, onStart: () => started = true));
    expect(find.text('Good day, Sara'), findsOneWidget);
    expect(find.text('2 new reels · 1 review'), findsOneWidget);
    expect(find.text('about 75 s'), findsOneWidget);
    expect(find.text('12-day streak'), findsOneWidget);
    expect(
      find.textContaining('Yesterday you remembered 2 of 3.'),
      findsOneWidget,
    );
    expect(find.text(Fixtures.reel1.title), findsOneWidget);
    await tester.tap(find.byKey(const Key('start-minute')));
    expect(started, isTrue);
    expect(find.textContaining('scroll'), findsNothing);
    expect(find.textContaining('feed'), findsNothing);
  });

  testWidgets('empty plan shows the calm empty state', (tester) async {
    await tester.pumpWidget(
      today(Fixtures.plan.copyWith(status: PlanStatus.empty, items: const [])),
    );
    expect(find.byKey(const Key('today-empty')), findsOneWidget);
    expect(find.text('Nothing for today'), findsOneWidget);
    expect(find.byKey(const Key('start-minute')), findsNothing);
  });

  testWidgets('done plan shows the done state', (tester) async {
    final done = Fixtures.plan.copyWith(
      status: PlanStatus.done,
      items: [
        for (final i in Fixtures.planItems)
          i.copyWith(state: PlanItemState.answered),
      ],
    );
    await tester.pumpWidget(today(done));
    expect(find.byKey(const Key('today-done')), findsOneWidget);
    expect(find.text('Done for today'), findsOneWidget);
    expect(find.text('Remembered 3 of 3'), findsOneWidget);
  });

  testWidgets('done summary: mint check, tally, streak, freeze note, actions',
      (tester) async {
    var back = false;
    var saved = false;
    await tester.pumpWidget(
      wrapApp(
        Scaffold(
          body: DoneSummary(
            remembered: 2,
            total: 3,
            stats: Fixtures.stats,
            onBackToWork: () => back = true,
            onOpenSaved: () => saved = true,
          ),
        ),
      ),
    );
    expect(find.byKey(const Key('done-check')), findsOneWidget);
    expect(find.text('Done for today'), findsOneWidget);
    expect(find.text('Remembered 2 of 3'), findsOneWidget);
    expect(find.text('12-day streak'), findsOneWidget);
    expect(find.textContaining('1 streak freeze left'), findsOneWidget);
    expect(find.textContaining('Fridays never count'), findsOneWidget);
    await tester.tap(find.byKey(const Key('back-to-work')));
    await tester.tap(find.byKey(const Key('open-saved')));
    expect(back && saved, isTrue);
  });
}
