import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:reelearn_mobile/core/api/fixtures.dart';
import 'package:reelearn_mobile/features/team/data/team_repository.dart';
import 'package:reelearn_mobile/features/team/presentation/team_screen.dart';

import '../../helpers.dart';

void main() {
  testWidgets('renders readiness per member per topic and team recall',
      (tester) async {
    tallViewport(tester);
    await tester.pumpWidget(
      wrapApp(Scaffold(body: TeamBody(team: Fixtures.team, onNudge: (_) {}))),
    );
    expect(find.text('Team recall 71%'), findsOneWidget);
    expect(find.text('3 of 4 active this week'), findsOneWidget);
    expect(find.text('Customer service · Ready'), findsNWidgets(2));
    expect(find.text('AML · Not started'), findsNWidgets(2));
    expect(find.text('Store safety · Not yet'), findsNWidgets(2));
    expect(
      find.byKey(const Key('member-${Fixtures.rizwanId}')),
      findsOneWidget,
    );
    expect(find.text('UR'), findsOneWidget);
    // Fatimah was nudged today: no button, a "Nudged today" pill.
    expect(find.byKey(const Key('nudge-${Fixtures.fatimahId}')), findsNothing);
    expect(
      find.byKey(const Key('nudged-${Fixtures.fatimahId}')),
      findsOneWidget,
    );
    expect(find.byKey(const Key('nudge-${Fixtures.rizwanId}')), findsOneWidget);
  });

  testWidgets('nudging sends once and disables the button for the day',
      (tester) async {
    tallViewport(tester);
    final env = TestEnv(signedInAs: Fixtures.khalid);
    await tester.pumpWidget(env.wrap(const TeamScreen()));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('nudge-${Fixtures.rizwanId}')));
    await tester.pumpAndSettle();
    expect(env.api.nudgedUserIds, [Fixtures.rizwanId]);
    expect(find.text('Reminder sent to Rizwan Ahmed'), findsOneWidget);
    expect(find.byKey(const Key('nudge-${Fixtures.rizwanId}')), findsNothing);
    expect(
      find.byKey(const Key('nudged-${Fixtures.rizwanId}')),
      findsOneWidget,
    );

    // A second attempt through the controller is refused locally and by the API.
    final ctrl = containerOf(tester).read(teamProvider.notifier);
    final again = await ctrl.nudge(Fixtures.rizwanId);
    expect(again?.queued, 0);
    expect(again?.skippedAlreadyNudged, 1);
    expect(env.api.nudgedUserIds.length, 1);
    final direct = await env.api.nudge([Fixtures.rizwanId, Fixtures.saraId]);
    expect(direct.queued, 1);
    expect(direct.skippedAlreadyNudged, 1);
  });
}
