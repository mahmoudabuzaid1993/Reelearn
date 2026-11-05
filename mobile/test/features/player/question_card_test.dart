import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:reelearn_mobile/app/theme.dart';
import 'package:reelearn_mobile/core/api/fixtures.dart';
import 'package:reelearn_mobile/core/api/models/models.dart';
import 'package:reelearn_mobile/features/player/domain/stack_model.dart';
import 'package:reelearn_mobile/features/player/presentation/question_card_view.dart';

import '../../helpers.dart';

const card = QuestionStackCard(
  item: PlanItem(
    id: Fixtures.item1Id,
    position: 1,
    kind: PlanItemKind.reel,
    reason: PlanItemReason.isNew,
    state: PlanItemState.pending,
  ),
  question: Fixtures.question1,
  unitIndex: 0,
  isReview: false,
);

class _Host extends StatefulWidget {
  const _Host({required this.onPick, this.initial});
  final void Function(String) onPick;
  final StackAnswer? initial;

  @override
  State<_Host> createState() => _HostState();
}

class _HostState extends State<_Host> {
  StackAnswer? answer;

  @override
  void initState() {
    super.initState();
    answer = widget.initial;
  }

  @override
  Widget build(BuildContext context) => Scaffold(
        body: QuestionCardView(
          card: card,
          answer: answer,
          onPick: (key) {
            if (answer != null) return;
            widget.onPick(key);
            setState(() {
              answer = StackAnswer(
                chosenKey: key,
                outcome: AnswerOutcome(
                  isCorrect: key == 'B',
                  correctKey: 'B',
                  explanation: Fixtures.explanations[Fixtures.question1Id],
                  nextDueOn: '2026-11-02',
                  planStatus: PlanStatus.open,
                ),
              );
            });
          },
        ),
      );
}

Color materialColorOf(WidgetTester tester, Key key) {
  final material = find.descendant(
    of: find.byKey(key),
    matching: find.byType(Material),
  );
  return tester.widget<Material>(material.first).color!;
}

void main() {
  testWidgets('pick once, then the right answer is mint and feedback shows',
      (tester) async {
    final picks = <String>[];
    await tester.pumpWidget(wrapApp(_Host(onPick: picks.add)));
    expect(find.text(Fixtures.question1.prompt), findsOneWidget);
    expect(find.byKey(const Key('feedback-box')), findsNothing);

    await tester.tap(find.byKey(const Key('option-C')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('option-A')));
    await tester.pumpAndSettle();
    expect(picks, ['C']);

    expect(materialColorOf(tester, const Key('option-B')), Brand.mint);
    expect(materialColorOf(tester, const Key('option-C')), Brand.white);
    expect(find.text('Not quite.'), findsOneWidget);
    expect(
      find.text('Twice is enough; more sounds like a script.'),
      findsOneWidget,
    );
    expect(find.textContaining('Comes back on'), findsOneWidget);
    expect(find.byIcon(Icons.close), findsNothing);
  });

  testWidgets('a correct pick says Remembered.', (tester) async {
    await tester.pumpWidget(wrapApp(_Host(onPick: (_) {})));
    await tester.tap(find.byKey(const Key('option-B')));
    await tester.pumpAndSettle();
    expect(find.text('Remembered.'), findsOneWidget);
    expect(materialColorOf(tester, const Key('option-B')), Brand.mint);
  });

  testWidgets('no red anywhere in the tree, before or after answering',
      (tester) async {
    await tester.pumpWidget(wrapApp(_Host(onPick: (_) {})));
    expect(visibleColors(tester).where(isReddish), isEmpty);
    await tester.tap(find.byKey(const Key('option-A')));
    await tester.pumpAndSettle();
    expect(visibleColors(tester).where(isReddish), isEmpty);
  });

  testWidgets('offline pick shows the saved note instead of a reveal',
      (tester) async {
    await tester.pumpWidget(
      wrapApp(
        _Host(
          onPick: (_) {},
          initial: const StackAnswer(chosenKey: 'A', pendingOffline: true),
        ),
      ),
    );
    expect(find.textContaining('Answer saved'), findsOneWidget);
    expect(find.text('Remembered.'), findsNothing);
  });
}
