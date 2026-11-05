import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../../app/l10n/gen/app_localizations.dart';
import '../../../app/router.dart';
import '../../../app/theme.dart';
import '../../../app/widgets.dart';
import '../../../core/auth/session.dart';
import '../../../core/errors/app_error.dart';
import '../../../core/providers.dart';
import '../../today/data/today_repository.dart';
import '../data/telemetry.dart';
import '../domain/stack_model.dart';
import 'swipe_stack.dart';
import 'video_reel_media.dart';

/// Where the learner is in today's stack, kept across rebuilds and across a
/// trip back to /today, so backgrounding or re-entering lands on the same card.
class PlayMemo {
  const PlayMemo({
    this.planId,
    this.index = 0,
    this.answers = const {},
  });

  final String? planId;
  final int index;
  final Map<String, StackAnswer> answers;

  PlayMemo copyWith({
    String? planId,
    int? index,
    Map<String, StackAnswer>? answers,
  }) =>
      PlayMemo(
        planId: planId ?? this.planId,
        index: index ?? this.index,
        answers: answers ?? this.answers,
      );
}

final playMemoProvider = StateProvider<PlayMemo>((_) => const PlayMemo());

class PlayScreen extends ConsumerWidget {
  const PlayScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final plan = ref.watch(todayProvider);
    final me = ref.watch(meProvider);

    return Scaffold(
      backgroundColor: Brand.night,
      body: plan.when(
        loading: () => const ColoredBox(color: Brand.night),
        error: (e, _) => SafeArea(
          child: Center(
            child: ErrorLine(
              error: e,
              onRetry: () => ref.invalidate(todayProvider),
            ),
          ),
        ),
        data: (p) {
          if (p.isEmpty) {
            return SafeArea(
              child: Center(
                child: Text(
                  l.emptyTitle,
                  style: Theme.of(context)
                      .textTheme
                      .titleLarge
                      ?.copyWith(color: Brand.white),
                ),
              ),
            );
          }
          final cards = buildStackCards(p.items);
          var memo = ref.read(playMemoProvider);
          if (memo.planId != p.id) {
            memo = PlayMemo(planId: p.id, answers: seedAnswers(p.items));
            ref.read(playMemoProvider.notifier).state = memo;
          }
          final telemetry = ref.read(telemetryProvider);
          final api = ref.read(apiProvider);

          return SwipeStack(
            cards: cards,
            stats: p.stats,
            initialIndex: memo.index,
            initialAnswers: memo.answers,
            captionsDefault: me?.captionsOn ?? true,
            mediaBuilder: videoReelMedia,
            onIndexChanged: (i) => ref.read(playMemoProvider.notifier).state =
                ref.read(playMemoProvider).copyWith(index: i),
            onArrive: (card, _) {
              if (card is ReelStackCard) {
                telemetry.recordWatch(
                  planItemId: card.item.id,
                  reelId: card.reel.id,
                  watchedMs: 0,
                  completed: false,
                );
              }
            },
            onWatched: (card, ms) => telemetry.recordWatch(
              planItemId: card.item.id,
              reelId: card.reel.id,
              watchedMs: ms,
              completed: true,
            ),
            onPick: (card, key) async {
              final outcome = await telemetry.answer(
                planItemId: card.item.id,
                questionId: card.question.id,
                chosenKey: key,
              );
              final memoNow = ref.read(playMemoProvider);
              ref.read(playMemoProvider.notifier).state = memoNow.copyWith(
                answers: {
                  ...memoNow.answers,
                  card.item.id: StackAnswer(
                    chosenKey: key,
                    outcome: outcome,
                    pendingOffline: outcome == null,
                  ),
                },
              );
              return outcome;
            },
            onToggleSave: (card) async {
              final wasSaved = card.reel.saved;
              try {
                if (wasSaved) {
                  await api.unsaveReel(card.reel.id);
                } else {
                  await api.saveReel(card.reel.id);
                }
                return !wasSaved;
              } on AppError {
                return wasSaved;
              }
            },
            onClose: () {
              ref.invalidate(todayProvider);
              final onDone =
                  ref.read(playMemoProvider).index == cards.length - 1;
              context.go(onDone ? Routes.done : Routes.today);
            },
            onBackToWork: () {
              ref.invalidate(todayProvider);
              context.go(Routes.today);
            },
            onOpenSaved: () {
              ref.invalidate(todayProvider);
              context.go(Routes.saved);
            },
          );
        },
      ),
    );
  }
}
