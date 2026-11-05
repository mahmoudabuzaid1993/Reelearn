import 'package:flutter/material.dart';

import '../../../app/format.dart';
import '../../../app/l10n/gen/app_localizations.dart';
import '../../../app/theme.dart';
import '../../../core/api/models/models.dart';
import '../domain/stack_model.dart';

/// A recall question. Pick once; then the right answer turns mint and a
/// feedback box explains. No red, no cross, no buzz, no score.
class QuestionCardView extends StatelessWidget {
  const QuestionCardView({
    required this.card,
    required this.answer,
    required this.onPick,
    super.key,
  });

  final QuestionStackCard card;
  final StackAnswer? answer;
  final void Function(String key) onPick;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final text = Theme.of(context).textTheme;
    final q = card.question;
    final a = answer;
    final answered = a != null;
    final outcome = a?.outcome;
    final reviewNote = card.reviewNote;

    return ColoredBox(
      color: Brand.paper,
      child: SafeArea(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(20, 72, 20, 24),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              if (card.isReview)
                Align(
                  alignment: AlignmentDirectional.centerStart,
                  child: Container(
                    key: const Key('review-chip'),
                    padding: const EdgeInsets.symmetric(
                      horizontal: 12,
                      vertical: 6,
                    ),
                    decoration: BoxDecoration(
                      color: Brand.white,
                      border: Border.all(color: Brand.border),
                      borderRadius: BorderRadius.circular(999),
                    ),
                    child: Text(
                      reviewNote ?? l.reviewLabel,
                      style: text.labelLarge?.copyWith(color: Brand.slate),
                    ),
                  ),
                ),
              const SizedBox(height: 16),
              Text(
                q.prompt,
                style:
                    text.headlineSmall?.copyWith(fontWeight: FontWeight.w600),
              ),
              const SizedBox(height: 24),
              Expanded(
                // Never scrollable: the stack owns the vertical gesture.
                child: SingleChildScrollView(
                  physics: const NeverScrollableScrollPhysics(),
                  child: Column(
                    children: [
                      for (final o in q.options) ...[
                        _OptionButton(
                          key: Key('option-${o.key}'),
                          option: o,
                          state: _stateFor(o.key, a),
                          onTap: answered ? null : () => onPick(o.key),
                        ),
                        const SizedBox(height: 12),
                      ],
                      if (answered) _Feedback(answer: a, l: l),
                    ],
                  ),
                ),
              ),
              if (answered && outcome != null)
                Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    const Icon(
                      Icons.keyboard_arrow_up,
                      color: Brand.slate,
                      size: 20,
                    ),
                    const SizedBox(width: 4),
                    Text(
                      l.swipeHintNext,
                      style: text.labelLarge?.copyWith(color: Brand.slate),
                    ),
                  ],
                ),
            ],
          ),
        ),
      ),
    );
  }

  static _OptionState _stateFor(String key, StackAnswer? a) {
    if (a == null) return _OptionState.idle;
    final outcome = a.outcome;
    if (outcome == null) {
      return a.chosenKey == key
          ? _OptionState.chosenPending
          : _OptionState.muted;
    }
    if (outcome.correctKey == key) return _OptionState.correct;
    return _OptionState.muted;
  }
}

enum _OptionState { idle, chosenPending, correct, muted }

class _OptionButton extends StatelessWidget {
  const _OptionButton({
    required this.option,
    required this.state,
    required this.onTap,
    super.key,
  });

  final QuestionOption option;
  final _OptionState state;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    final (bg, border, fg) = switch (state) {
      _OptionState.idle => (Brand.white, Brand.border, Brand.ink),
      _OptionState.chosenPending => (Brand.white, Brand.ink, Brand.ink),
      _OptionState.correct => (Brand.mint, Brand.mint, Brand.night),
      _OptionState.muted => (Brand.white, Brand.border, Brand.slate),
    };
    return Semantics(
      button: true,
      selected: state == _OptionState.correct,
      child: Material(
        color: bg,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(Brand.radius),
          side: BorderSide(color: border, width: 1.5),
        ),
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(Brand.radius),
          child: ConstrainedBox(
            constraints: const BoxConstraints(minHeight: 60),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 16),
              child: Row(
                children: [
                  Container(
                    width: 28,
                    height: 28,
                    alignment: Alignment.center,
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      border: Border.all(color: fg),
                    ),
                    child: state == _OptionState.correct
                        ? Icon(Icons.check, size: 18, color: fg)
                        : Text(
                            option.key,
                            style: text.labelLarge?.copyWith(color: fg),
                          ),
                  ),
                  const SizedBox(width: 14),
                  Expanded(
                    child: Text(
                      option.text,
                      style: text.titleMedium?.copyWith(color: fg),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _Feedback extends StatelessWidget {
  const _Feedback({required this.answer, required this.l});
  final StackAnswer answer;
  final AppLocalizations l;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    final outcome = answer.outcome;

    if (outcome == null) {
      return Container(
        key: const Key('feedback-box'),
        width: double.infinity,
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(
          color: Brand.white,
          borderRadius: BorderRadius.circular(Brand.radius),
          border: Border.all(color: Brand.border),
        ),
        child: Text(
          answer.pendingOffline ? l.answerSavedOffline : '',
          style: text.bodyMedium?.copyWith(color: Brand.slate),
        ),
      );
    }

    return Container(
      key: const Key('feedback-box'),
      width: double.infinity,
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: Brand.white,
        borderRadius: BorderRadius.circular(Brand.radius),
        border:
            Border.all(color: outcome.isCorrect ? Brand.mint : Brand.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            outcome.isCorrect ? l.remembered : l.notQuite,
            style: text.titleMedium?.copyWith(
              fontWeight: FontWeight.w600,
              color: outcome.isCorrect ? Brand.mint : Brand.ink,
            ),
          ),
          if (outcome.explanation != null) ...[
            const SizedBox(height: 6),
            Text(outcome.explanation!, style: text.bodyMedium),
          ],
          const SizedBox(height: 6),
          Text(
            l.comesBackOn(formatIsoDate(context, outcome.nextDueOn)),
            style: text.bodySmall?.copyWith(color: Brand.slate),
          ),
        ],
      ),
    );
  }
}
