import 'package:flutter_test/flutter_test.dart';
import 'package:reelearn_mobile/core/api/fixtures.dart';
import 'package:reelearn_mobile/core/api/models/models.dart';
import 'package:reelearn_mobile/features/player/domain/stack_model.dart';

void main() {
  final cards = buildStackCards(Fixtures.planItems);

  test('builds reel+question per reel item, one card per review, done last',
      () {
    expect(cards.length, 6);
    expect(cards[0], isA<ReelStackCard>());
    expect(cards[1], isA<QuestionStackCard>());
    expect(cards[2], isA<ReelStackCard>());
    expect(cards[3], isA<QuestionStackCard>());
    expect(cards[4], isA<QuestionStackCard>());
    expect((cards[4] as QuestionStackCard).isReview, isTrue);
    expect(cards.last, isA<DoneStackCard>());
    expect(unitCount(cards), 3);
  });

  test('forward is gated on an unanswered question, never on the done card',
      () {
    expect(canGoForward(cards: cards, index: 0, answers: const {}), isTrue);
    expect(canGoForward(cards: cards, index: 1, answers: const {}), isFalse);
    expect(
      isGatedByQuestion(cards: cards, index: 1, answers: const {}),
      isTrue,
    );
    final answered = {Fixtures.item1Id: const StackAnswer(chosenKey: 'B')};
    expect(canGoForward(cards: cards, index: 1, answers: answered), isTrue);
    expect(canGoForward(cards: cards, index: 5, answers: answered), isFalse);
    expect(canGoBackward(0), isFalse);
    expect(canGoBackward(1), isTrue);
  });

  test('segments: upcoming / active / remembered / answered', () {
    expect(
      computeSegments(cards: cards, answers: const {}, currentIndex: 0),
      [SegmentState.active, SegmentState.upcoming, SegmentState.upcoming],
    );
    const right = AnswerOutcome(
      isCorrect: true,
      correctKey: 'B',
      nextDueOn: '2026-11-02',
      planStatus: PlanStatus.open,
    );
    const wrong = AnswerOutcome(
      isCorrect: false,
      correctKey: 'B',
      nextDueOn: '2026-11-02',
      planStatus: PlanStatus.open,
    );
    final answers = {
      Fixtures.item1Id: const StackAnswer(chosenKey: 'B', outcome: right),
      Fixtures.item2Id: const StackAnswer(chosenKey: 'A', outcome: wrong),
    };
    expect(
      computeSegments(cards: cards, answers: answers, currentIndex: 4),
      [SegmentState.remembered, SegmentState.answered, SegmentState.active],
    );
    expect(rememberedTally(cards, answers), (remembered: 1, total: 3));
  });

  test('seedAnswers marks items the API already answered', () {
    final items = [
      Fixtures.planItems[0].copyWith(state: PlanItemState.answered),
      Fixtures.planItems[1],
    ];
    expect(seedAnswers(items).keys, [Fixtures.item1Id]);
  });
}
