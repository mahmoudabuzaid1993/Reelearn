import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../../app/router.dart';
import '../../../app/widgets.dart';
import '../../../core/api/models/models.dart';
import '../../player/domain/stack_model.dart';
import '../../player/presentation/play_screen.dart';
import '../data/today_repository.dart';
import 'done_summary.dart';

/// `/today/done`: the summary after the minute, from the plan plus whatever
/// outcomes this session remembers.
class DoneScreen extends ConsumerWidget {
  const DoneScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final plan = ref.watch(todayProvider);
    final memo = ref.watch(playMemoProvider);

    return Scaffold(
      body: plan.when(
        loading: () => const SkeletonList(rows: 2),
        error: (e, _) =>
            ErrorLine(error: e, onRetry: () => ref.invalidate(todayProvider)),
        data: (p) {
          final cards = buildStackCards(p.items);
          final tally = rememberedTally(cards, memo.answers);
          final total =
              p.items.where((i) => i.effectiveQuestion != null).length;
          final latestStats = memo.answers.values
                  .map((a) => a.outcome?.stats)
                  .whereType<UserStats>()
                  .lastOrNull ??
              p.stats;
          return DoneSummary(
            remembered: tally.remembered,
            total: total,
            stats: latestStats,
            onBackToWork: () => context.go(Routes.today),
            onOpenSaved: () => context.go(Routes.saved),
          );
        },
      ),
    );
  }
}
