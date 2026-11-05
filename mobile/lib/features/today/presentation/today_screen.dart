import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../../app/format.dart';
import '../../../app/l10n/gen/app_localizations.dart';
import '../../../app/router.dart';
import '../../../app/theme.dart';
import '../../../app/widgets.dart';
import '../../../core/api/models/models.dart';
import '../../../core/auth/session.dart';
import '../data/today_repository.dart';

class TodayScreen extends ConsumerWidget {
  const TodayScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final plan = ref.watch(todayProvider);
    final me = ref.watch(meProvider);

    return Scaffold(
      appBar: AppBar(title: Text(l.todayTitle)),
      body: RefreshIndicator(
        color: Brand.ink,
        onRefresh: () => ref.refresh(todayProvider.future),
        child: plan.when(
          loading: () => const SkeletonList(rows: 3),
          error: (e, _) => ListView(
            children: [
              ErrorLine(error: e, onRetry: () => ref.invalidate(todayProvider)),
            ],
          ),
          data: (p) => TodayBody(
            plan: p,
            name: p.greetingName ?? me?.displayFirstName ?? '',
            onStart: () => context.go(Routes.play),
            onOpenSaved: () => context.go(Routes.saved),
            onOpenDone: () => context.go(Routes.done),
          ),
        ),
      ),
    );
  }
}

/// Pure body so tests can render the three states without providers.
class TodayBody extends StatelessWidget {
  const TodayBody({
    required this.plan,
    required this.name,
    required this.onStart,
    required this.onOpenSaved,
    required this.onOpenDone,
    super.key,
  });

  final DailyPlan plan;
  final String name;
  final VoidCallback onStart;
  final VoidCallback onOpenSaved;
  final VoidCallback onOpenDone;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final text = Theme.of(context).textTheme;
    final stats = plan.stats;

    final header = Padding(
      padding: const EdgeInsets.fromLTRB(20, 8, 20, 0),
      child: Row(
        children: [
          Expanded(
            child: Text(
              l.greeting(name),
              style: text.headlineSmall?.copyWith(fontWeight: FontWeight.w600),
            ),
          ),
          Pill(
            key: const Key('streak-chip'),
            icon: Icons.local_fire_department_outlined,
            label: l.streakChip(stats.streakCurrent),
          ),
        ],
      ),
    );

    final yesterday = plan.yesterday;
    final yesterdayLine = yesterday == null
        ? null
        : Padding(
            padding: const EdgeInsets.fromLTRB(20, 16, 20, 0),
            child: Text(
              [
                l.yesterdayLine(yesterday.remembered, yesterday.total),
                if (yesterday.nextReviewOn != null)
                  l.nextReviewOn(
                    formatIsoDate(context, yesterday.nextReviewOn),
                  ),
              ].join(' '),
              key: const Key('yesterday-line'),
              style: text.bodyMedium?.copyWith(color: Brand.slate),
            ),
          );

    Widget body;
    if (plan.isEmpty) {
      body = _StateCard(
        key: const Key('today-empty'),
        icon: Icons.wb_sunny_outlined,
        title: l.emptyTitle,
        body: l.emptyBody,
        action:
            OutlinedButton(onPressed: onOpenSaved, child: Text(l.openSaved)),
      );
    } else if (plan.isDone) {
      body = _StateCard(
        key: const Key('today-done'),
        icon: Icons.check_circle_outline,
        iconColor: Brand.mint,
        title: l.doneTitle,
        body: l.doneBody,
        action: Column(
          children: [
            OutlinedButton(
              onPressed: onOpenDone,
              child: Text(
                l.doneRemembered(
                  plan.items
                      .where((i) => i.state == PlanItemState.answered)
                      .length,
                  plan.items.where((i) => i.effectiveQuestion != null).length,
                ),
              ),
            ),
            const SizedBox(height: 8),
            TextButton(onPressed: onOpenSaved, child: Text(l.openSaved)),
          ],
        ),
      );
    } else {
      final started = plan.items.any((i) => i.state != PlanItemState.pending);
      body = Card(
        key: const Key('today-plan'),
        child: Padding(
          padding: const EdgeInsets.all(20),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                '${l.newReelsCount(plan.newReels)} · ${l.reviewsCount(plan.reviews)}',
                style: text.titleLarge?.copyWith(fontWeight: FontWeight.w600),
              ),
              const SizedBox(height: 4),
              Text(
                l.aboutSeconds(plan.estimatedSeconds),
                style: text.bodyMedium?.copyWith(color: Brand.slate),
              ),
              const SizedBox(height: 16),
              for (final item in plan.items) _PlanRow(item: item),
              const SizedBox(height: 16),
              FilledButton(
                key: const Key('start-minute'),
                onPressed: onStart,
                child: Text(started ? l.continueMinute : l.startMinute),
              ),
            ],
          ),
        ),
      );
    }

    return ListView(
      padding: const EdgeInsets.only(bottom: 24),
      children: [
        header,
        if (yesterdayLine != null) yesterdayLine,
        Padding(padding: const EdgeInsets.fromLTRB(20, 20, 20, 0), child: body),
      ],
    );
  }
}

class _PlanRow extends StatelessWidget {
  const _PlanRow({required this.item});
  final PlanItem item;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final text = Theme.of(context).textTheme;
    final title = item.reel?.title ?? item.effectiveQuestion?.prompt ?? '';
    final topic = item.reel?.topicName ?? (item.reviewNote ?? l.reviewLabel);
    final done = item.state == PlanItemState.answered;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(
            done
                ? Icons.check_circle
                : item.kind == PlanItemKind.reel
                    ? Icons.play_circle_outline
                    : Icons.help_outline,
            size: 22,
            color: done ? Brand.mint : Brand.slate,
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(title, style: text.bodyLarge),
                Text(
                  topic,
                  style: text.bodySmall?.copyWith(color: Brand.slate),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _StateCard extends StatelessWidget {
  const _StateCard({
    required this.icon,
    required this.title,
    required this.body,
    required this.action,
    this.iconColor = Brand.slate,
    super.key,
  });

  final IconData icon;
  final Color iconColor;
  final String title;
  final String body;
  final Widget action;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(icon, size: 40, color: iconColor),
            const SizedBox(height: 16),
            Text(
              title,
              style: text.titleLarge?.copyWith(fontWeight: FontWeight.w600),
            ),
            const SizedBox(height: 8),
            Text(body, style: text.bodyMedium?.copyWith(color: Brand.slate)),
            const SizedBox(height: 20),
            action,
          ],
        ),
      ),
    );
  }
}
