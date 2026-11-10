import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:reelearn_mobile/core/api/fixtures.dart';
import 'package:reelearn_mobile/core/api/models/models.dart';
import 'package:reelearn_mobile/features/player/domain/stack_model.dart';
import 'package:reelearn_mobile/features/player/presentation/reel_card_view.dart';
import 'package:reelearn_mobile/features/practice/application/practice_controller.dart';
import 'package:reelearn_mobile/features/practice/presentation/practice_screen.dart';

import '../../helpers.dart';

Future<void> settle() async {
  for (var i = 0; i < 8; i++) {
    await Future<void>.delayed(Duration.zero);
  }
}

ProviderContainer containerFor(TestEnv env) {
  final c = ProviderContainer(overrides: env.overrides);
  addTearDown(c.dispose);
  return c;
}

void main() {
  group('practice controller', () {
    test(
        'first batch: a colleague share leads, items are not plan items, a review is interleaved',
        () async {
      final env = TestEnv(signedInAs: Fixtures.sara);
      final c = containerFor(env);
      final sub = c.listen(practiceProvider, (_, __) {});
      addTearDown(sub.close);
      await c.read(practiceProvider.notifier).start();
      final s = c.read(practiceProvider);
      expect(s.sessionId, isNotNull);
      final first = s.cards.first as ReelStackCard;
      expect(first.item.practiceReason, 'shared');
      expect(first.item.reasonDetail, 'Fatimah');
      expect(first.item.inPlan, isFalse);
      expect(first.item.planItemId, isNull);
      expect(
        s.cards.whereType<QuestionStackCard>().any((q) => q.isReview),
        isTrue,
      );
      expect(s.cards.last, isA<LoadingStackCard>());
    });

    test('loading until the catalogue runs out ends with an end card',
        () async {
      final env = TestEnv(signedInAs: Fixtures.sara);
      final c = containerFor(env);
      final sub = c.listen(practiceProvider, (_, __) {});
      addTearDown(sub.close);
      final ctrl = c.read(practiceProvider.notifier);
      await ctrl.start();
      for (var i = 0; i < 5 && !c.read(practiceProvider).ended; i++) {
        await ctrl.loadMore();
      }
      final s = c.read(practiceProvider);
      expect(s.ended, isTrue);
      expect(
        (s.cards.last as PracticeEndStackCard).stop,
        PracticeStop.exhausted,
      );
      final ids =
          s.cards.whereType<ReelStackCard>().map((r) => r.reel.id).toList();
      expect(
        ids.toSet().length,
        ids.length,
        reason: 'no reel twice in a session',
      );
    });

    test('a company with practice off shows the end card straight away',
        () async {
      final env = TestEnv(signedInAs: Fixtures.sara)..api.practiceOpen = false;
      final c = containerFor(env);
      final sub = c.listen(practiceProvider, (_, __) {});
      addTearDown(sub.close);
      await c.read(practiceProvider.notifier).start();
      final s = c.read(practiceProvider);
      expect(s.cards, hasLength(1));
      expect(
        (s.cards.single as PracticeEndStackCard).stop,
        PracticeStop.disabled,
      );
    });

    test(
        'practice time is reported, adds a pause card, and the session cap closes practice',
        () async {
      final env = TestEnv(signedInAs: Fixtures.sara);
      env.api
        ..breakEveryS = 60
        ..sessionCapS = 60;
      final c = containerFor(env);
      final sub = c.listen(practiceProvider, (_, __) {});
      addTearDown(sub.close);
      final ctrl = c.read(practiceProvider.notifier);
      await ctrl.start();
      ctrl.onIndex(0);
      await ctrl.addActive(61000);
      await settle();
      final s = c.read(practiceProvider);
      expect(s.cards[1], isA<BreakStackCard>());
      expect(env.api.engagementBatches.single.activeMs, 61000);
      expect(env.api.engagementBatches.single.sessionId, s.sessionId);
      await ctrl.loadMore();
      final after = c.read(practiceProvider);
      expect(
        (after.cards.last as PracticeEndStackCard).stop,
        PracticeStop.sessionCap,
      );
    });

    test('signals go to /me/events with the session id', () async {
      final env = TestEnv(signedInAs: Fixtures.sara);
      final c = containerFor(env);
      final sub = c.listen(practiceProvider, (_, __) {});
      addTearDown(sub.close);
      final ctrl = c.read(practiceProvider.notifier);
      await ctrl.start();
      await ctrl.signal(
        const EngagementSignal(
          reelId: Fixtures.reel2Id,
          kind: EngagementKind.notRelevant,
        ),
      );
      await settle();
      final batch = env.api.engagementBatches.single;
      expect(batch.events.single.kind, EngagementKind.notRelevant);
      expect(batch.toJson()['events'], [
        {'reel_id': Fixtures.reel2Id, 'kind': 'not_relevant'},
      ]);
    });
  });

  group('practice screen', () {
    testWidgets(
        'shows why a reel is there, "not for me" moves on, and sending works',
        (tester) async {
      final env = TestEnv(signedInAs: Fixtures.sara);
      await tester.pumpWidget(
        env.wrap(const PracticeScreen(mediaBuilder: placeholderReelMedia)),
      );
      await tester.pumpAndSettle();
      expect(find.text('Sent by Fatimah'), findsOneWidget);
      expect(find.text(Fixtures.reel2.title), findsOneWidget);

      await tester.tap(find.byKey(const Key('action-share')).first);
      await tester.pumpAndSettle();
      expect(find.text('Send to colleagues'), findsOneWidget);
      await tester.tap(find.byKey(const Key('share-${Fixtures.rizwanId}')));
      await tester.pump();
      await tester.tap(find.byKey(const Key('share-send')));
      await tester.pumpAndSettle();
      expect(env.api.shares.single.to, [Fixtures.rizwanId]);
      expect(find.text('Sent to 1 colleague'), findsOneWidget);

      await tester.tap(find.byKey(const Key('action-not-for-me')).first);
      await tester.pumpAndSettle();
      expect(find.text('Got it. Fewer reels like this.'), findsOneWidget);
      // The shared reel and its question were skipped: the next unit is on screen.
      final next = Fixtures.practice.first.reel.title;
      expect(find.text(next), findsWidgets);
      await tester.pump(const Duration(seconds: 5));
      expect(
        env.api.engagementBatches.expand((b) => b.events).map((e) => e.kind),
        contains(EngagementKind.notRelevant),
      );

      await tester.pumpWidget(const SizedBox());
    });
  });
}
