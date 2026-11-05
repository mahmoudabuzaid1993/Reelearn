import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:reelearn_mobile/app/theme.dart';
import 'package:reelearn_mobile/core/api/fixtures.dart';
import 'package:reelearn_mobile/core/api/models/models.dart';
import 'package:reelearn_mobile/features/player/domain/stack_model.dart';
import 'package:reelearn_mobile/features/player/presentation/reel_card_view.dart';
import 'package:reelearn_mobile/features/player/presentation/swipe_stack.dart';

import '../../helpers.dart';

AnswerOutcome outcomeFor(QuestionStackCard card, String key) {
  final correct = Fixtures.correctKeys[card.question.id]!;
  return AnswerOutcome(
    isCorrect: key == correct,
    correctKey: correct,
    explanation: Fixtures.explanations[card.question.id],
    nextDueOn: '2026-11-02',
    planStatus: PlanStatus.open,
  );
}

class Harness {
  final cards = buildStackCards(Fixtures.planItems);
  final arrivals = <int>[];
  final picks = <String>[];
  final watched = <String>[];
  var closed = false;

  Widget build({
    bool? reducedMotion,
    Locale locale = const Locale('en'),
    ReelMediaBuilder? mediaBuilder,
    int initialIndex = 0,
  }) =>
      wrapApp(
        Scaffold(
          body: SwipeStack(
            cards: cards,
            stats: Fixtures.stats,
            reducedMotion: reducedMotion,
            initialIndex: initialIndex,
            mediaBuilder: mediaBuilder,
            onArrive: (_, i) => arrivals.add(i),
            onWatched: (card, _) => watched.add(card.item.id),
            onPick: (card, key) async {
              picks.add(key);
              return outcomeFor(card, key);
            },
            onClose: () => closed = true,
            onBackToWork: () {},
            onOpenSaved: () {},
          ),
        ),
        locale: locale,
      );
}

final stack = find.byKey(const Key('swipe-stack'));

/// The option button on the card the learner is looking at (neighbour cards are built too).
Finder option(Harness h, String key) => find.descendant(
      of: find.byKey(Key('stack-card-${h.cards[h.arrivals.last].key}')),
      matching: find.byKey(Key('option-$key')),
    );

Future<void> swipeUp(WidgetTester tester) async {
  await tester.drag(stack, const Offset(0, -260));
  await tester.pumpAndSettle();
}

Future<void> swipeDown(WidgetTester tester) async {
  await tester.drag(stack, const Offset(0, 260));
  await tester.pumpAndSettle();
}

Color segmentColor(WidgetTester tester, int i) {
  final c = tester.widget<Container>(find.byKey(Key('segment-$i')));
  return (c.decoration! as BoxDecoration).color!;
}

void main() {
  testWidgets('renders the first reel card and fires arrival', (tester) async {
    final h = Harness();
    await tester.pumpWidget(h.build());
    await tester.pumpAndSettle();
    expect(find.text(Fixtures.reel1.title), findsOneWidget);
    expect(find.text('Swipe up for the question'), findsOneWidget);
    expect(find.byKey(const Key('reel-caption')), findsOneWidget);
    expect(h.arrivals, [0]);
    expect(find.byKey(const Key('stack-close')), findsOneWidget);
  });

  testWidgets('forward on an unanswered question shows the toast and stays',
      (tester) async {
    final h = Harness();
    await tester.pumpWidget(h.build());
    await tester.pumpAndSettle();
    await swipeUp(tester);
    expect(find.text(Fixtures.question1.prompt), findsOneWidget);
    expect(h.arrivals, [0, 1]);

    await tester.drag(stack, const Offset(0, -260));
    await tester.pump();
    expect(find.byKey(const Key('stack-toast')), findsOneWidget);
    expect(find.text('Answer first, then swipe up'), findsOneWidget);
    await tester.pumpAndSettle();
    // Still on the question; reel 2 never became active.
    expect(find.text(Fixtures.question1.prompt), findsOneWidget);
    expect(h.arrivals, [0, 1]);
  });

  testWidgets('an answered question allows forward; pick is once',
      (tester) async {
    final h = Harness();
    await tester.pumpWidget(h.build());
    await tester.pumpAndSettle();
    await swipeUp(tester);
    await tester.tap(option(h, 'B'));
    await tester.pumpAndSettle();
    await tester.tap(option(h, 'A'));
    await tester.pumpAndSettle();
    expect(h.picks, ['B']);
    expect(find.text('Remembered.'), findsOneWidget);

    await swipeUp(tester);
    expect(find.text(Fixtures.reel2.title), findsOneWidget);
    expect(h.arrivals, [0, 1, 2]);
  });

  testWidgets('backward is always allowed, even from an unanswered question',
      (tester) async {
    final h = Harness();
    await tester.pumpWidget(h.build());
    await tester.pumpAndSettle();
    await swipeUp(tester);
    await swipeDown(tester);
    expect(h.arrivals, [0, 1, 0]);
    expect(find.text(Fixtures.reel1.title), findsOneWidget);
    // Rubber band at the very start: a backward swipe on card 0 stays.
    await swipeDown(tester);
    expect(h.arrivals, [0, 1, 0]);
  });

  testWidgets('done card is last and nothing follows it', (tester) async {
    final h = Harness();
    await tester.pumpWidget(h.build());
    await tester.pumpAndSettle();
    await swipeUp(tester); // q1
    await tester.tap(option(h, 'B'));
    await tester.pumpAndSettle();
    await swipeUp(tester); // reel 2
    await swipeUp(tester); // q2
    await tester.tap(option(h, 'A'));
    await tester.pumpAndSettle();
    expect(find.text('Not quite.'), findsOneWidget);
    await swipeUp(tester); // review
    expect(find.byKey(const Key('review-chip')), findsOneWidget);
    expect(find.text('Asked again · day 10'), findsOneWidget);
    await tester.tap(option(h, 'A'));
    await tester.pumpAndSettle();
    await swipeUp(tester); // done
    expect(find.text('Done for today'), findsOneWidget);
    expect(find.text('Remembered 2 of 3'), findsOneWidget);
    expect(find.text('12-day streak'), findsOneWidget);
    expect(find.byKey(const Key('done-check')), findsOneWidget);
    expect(h.arrivals.last, h.cards.length - 1);
    expect(h.cards.last, isA<DoneStackCard>());

    await swipeUp(tester);
    expect(find.text('Done for today'), findsOneWidget);
    expect(h.arrivals.last, h.cards.length - 1);
    expect(h.arrivals.where((i) => i == h.cards.length - 1).length, 1);
  });

  testWidgets('reduced motion snaps instantly without animation',
      (tester) async {
    final h = Harness();
    await tester.pumpWidget(h.build(reducedMotion: true));
    await tester.pump();
    await tester.drag(stack, const Offset(0, -260));
    await tester.pump();
    expect(h.arrivals, [0, 1]);
    expect(find.text(Fixtures.question1.prompt), findsOneWidget);
    expect(tester.binding.hasScheduledFrame, isFalse);
  });

  testWidgets('segments show upcoming / active / remembered', (tester) async {
    final h = Harness();
    await tester.pumpWidget(h.build());
    await tester.pumpAndSettle();
    expect(segmentColor(tester, 0), Brand.white);
    expect(segmentColor(tester, 1), Brand.white.withValues(alpha: 0.3));
    expect(segmentColor(tester, 2), Brand.white.withValues(alpha: 0.3));

    await swipeUp(tester);
    await tester.tap(option(h, 'B'));
    await tester.pumpAndSettle();
    await swipeUp(tester);
    expect(segmentColor(tester, 0), Brand.mint);
    expect(segmentColor(tester, 1), Brand.white);
    expect(segmentColor(tester, 2), Brand.white.withValues(alpha: 0.3));
  });

  testWidgets('90 % playback fires onWatched once', (tester) async {
    final h = Harness();
    var fired = false;
    await tester.pumpWidget(
      h.build(
        mediaBuilder: (context, req) {
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (req.isActive && !fired) {
              fired = true;
              req.onProgress(41000, 45000);
              req.onProgress(42000, 45000);
            }
          });
          return const SizedBox.expand();
        },
      ),
    );
    await tester.pumpAndSettle();
    expect(h.watched, [Fixtures.item1Id]);
  });

  testWidgets('close calls onClose', (tester) async {
    final h = Harness();
    await tester.pumpWidget(h.build());
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('stack-close')));
    expect(h.closed, isTrue);
  });

  testWidgets('Arabic mirrors the layout; the vertical gesture is unchanged',
      (tester) async {
    final h = Harness();
    await tester.pumpWidget(h.build(locale: const Locale('ar')));
    await tester.pumpAndSettle();
    final ctx = tester.element(stack);
    expect(Directionality.of(ctx), TextDirection.rtl);
    // Side actions sit at the "end" edge: on the left under RTL.
    final save = tester.getCenter(find.byKey(const Key('action-save')));
    expect(save.dx, lessThan(400));
    await swipeUp(tester);
    expect(h.arrivals, [0, 1]);
    expect(find.text('أجب أولًا، ثم اسحب للأعلى'), findsNothing);
    await tester.drag(stack, const Offset(0, -260));
    await tester.pump();
    expect(find.text('أجب أولًا، ثم اسحب للأعلى'), findsOneWidget);
    await tester.pumpAndSettle();
  });
}
