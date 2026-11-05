import '../../../core/api/models/models.dart';

/// One full-screen card of the swipe stack.
sealed class StackCard {
  const StackCard();

  /// Stable key for widget identity and tests.
  String get key;

  /// Index of the unit this card belongs to (reel + its question = one unit,
  /// a review = one unit). The done card has none.
  int? get unitIndex;
}

class ReelStackCard extends StackCard {
  const ReelStackCard({
    required this.item,
    required this.reel,
    required this.unitIndex,
    required this.hasQuestion,
  });

  final PlanItem item;
  final ReelPlayable reel;
  @override
  final int unitIndex;
  final bool hasQuestion;

  @override
  String get key => 'reel:${item.id}';
}

class QuestionStackCard extends StackCard {
  const QuestionStackCard({
    required this.item,
    required this.question,
    required this.unitIndex,
    required this.isReview,
    this.reviewNote,
  });

  final PlanItem item;
  final Question question;
  @override
  final int unitIndex;
  final bool isReview;
  final String? reviewNote;

  @override
  String get key => 'question:${item.id}';
}

class DoneStackCard extends StackCard {
  const DoneStackCard();

  @override
  String get key => 'done';

  @override
  int? get unitIndex => null;
}

/// What the learner picked on a question card and what the API said.
class StackAnswer {
  const StackAnswer({
    required this.chosenKey,
    this.outcome,
    this.pendingOffline = false,
  });

  final String chosenKey;
  final AnswerOutcome? outcome;

  /// Picked while offline: queued, no reveal until the API answers.
  final bool pendingOffline;

  bool get isRemembered => outcome?.isCorrect ?? false;

  StackAnswer copyWith({AnswerOutcome? outcome, bool? pendingOffline}) =>
      StackAnswer(
        chosenKey: chosenKey,
        outcome: outcome ?? this.outcome,
        pendingOffline: pendingOffline ?? this.pendingOffline,
      );
}

enum SegmentState { upcoming, active, answered, remembered }

/// Builds the card list for a plan. Always ends with the done card.
List<StackCard> buildStackCards(List<PlanItem> items) {
  final sorted = List<PlanItem>.of(items)
    ..sort((a, b) => a.position.compareTo(b.position));
  final cards = <StackCard>[];
  var unit = 0;
  for (final item in sorted) {
    switch (item.kind) {
      case PlanItemKind.reel:
        final reel = item.reel;
        if (reel == null) continue;
        final q = item.effectiveQuestion;
        cards.add(
          ReelStackCard(
            item: item,
            reel: reel,
            unitIndex: unit,
            hasQuestion: q != null,
          ),
        );
        if (q != null) {
          cards.add(
            QuestionStackCard(
              item: item,
              question: q,
              unitIndex: unit,
              isReview: item.reason == PlanItemReason.review,
            ),
          );
        }
        unit++;
      case PlanItemKind.questionOnly:
        final q = item.effectiveQuestion;
        if (q == null) continue;
        cards.add(
          QuestionStackCard(
            item: item,
            question: q,
            unitIndex: unit,
            isReview: true,
            reviewNote: item.reviewNote,
          ),
        );
        unit++;
    }
  }
  cards.add(const DoneStackCard());
  return List.unmodifiable(cards);
}

int unitCount(List<StackCard> cards) {
  var max = -1;
  for (final c in cards) {
    final u = c.unitIndex;
    if (u != null && u > max) max = u;
  }
  return max + 1;
}

/// Items the API already marked answered (resumed mid-day) count as answered
/// even though the phone never saw the outcome.
Map<String, StackAnswer> seedAnswers(List<PlanItem> items) => {
      for (final i in items)
        if (i.state == PlanItemState.answered)
          i.id: const StackAnswer(chosenKey: ''),
    };

/// Forward is refused on an unanswered question and on the done card.
bool canGoForward({
  required List<StackCard> cards,
  required int index,
  required Map<String, StackAnswer> answers,
}) {
  if (index < 0 || index >= cards.length - 1) return false;
  final card = cards[index];
  if (card is QuestionStackCard && !answers.containsKey(card.item.id)) {
    return false;
  }
  return true;
}

/// The reason forward was refused, for the toast.
bool isGatedByQuestion({
  required List<StackCard> cards,
  required int index,
  required Map<String, StackAnswer> answers,
}) {
  if (index < 0 || index >= cards.length) return false;
  final card = cards[index];
  return card is QuestionStackCard && !answers.containsKey(card.item.id);
}

bool canGoBackward(int index) => index > 0;

/// One segment per unit: upcoming / active / answered / remembered.
List<SegmentState> computeSegments({
  required List<StackCard> cards,
  required Map<String, StackAnswer> answers,
  required int currentIndex,
}) {
  final units = unitCount(cards);
  final current = currentIndex >= 0 && currentIndex < cards.length
      ? cards[currentIndex].unitIndex
      : null;
  final onDone = currentIndex == cards.length - 1;

  return List.generate(units, (u) {
    final unitCards = cards.where((c) => c.unitIndex == u).toList();
    final question = unitCards.whereType<QuestionStackCard>().firstOrNull;
    if (question != null) {
      final a = answers[question.item.id];
      if (a != null) {
        return a.isRemembered ? SegmentState.remembered : SegmentState.answered;
      }
    } else {
      // A reel without a question is done once the learner has moved past it.
      final lastIndex = cards.lastIndexWhere((c) => c.unitIndex == u);
      if (onDone || currentIndex > lastIndex) return SegmentState.answered;
    }
    if (current == u) return SegmentState.active;
    return SegmentState.upcoming;
  });
}

/// Count of remembered units and total units with a question.
({int remembered, int total}) rememberedTally(
  List<StackCard> cards,
  Map<String, StackAnswer> answers,
) {
  var total = 0;
  var remembered = 0;
  for (final c in cards.whereType<QuestionStackCard>()) {
    total++;
    if (answers[c.item.id]?.isRemembered ?? false) remembered++;
  }
  return (remembered: remembered, total: total);
}
