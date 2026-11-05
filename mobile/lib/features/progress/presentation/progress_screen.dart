import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../app/format.dart';
import '../../../app/l10n/gen/app_localizations.dart';
import '../../../app/theme.dart';
import '../../../app/widgets.dart';
import '../../../core/api/models/models.dart';
import '../data/progress_repository.dart';

class ProgressScreen extends ConsumerWidget {
  const ProgressScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final progress = ref.watch(progressProvider);
    return Scaffold(
      appBar: AppBar(title: Text(l.progressTitle)),
      body: progress.when(
        loading: () => const SkeletonList(),
        error: (e, _) => ErrorLine(
          error: e,
          onRetry: () => ref.invalidate(progressProvider),
        ),
        data: (p) => ProgressBody(progress: p),
      ),
    );
  }
}

/// The headline is always "remembered", never watch minutes.
class ProgressBody extends StatelessWidget {
  const ProgressBody({required this.progress, super.key});
  final Progress progress;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final text = Theme.of(context).textTheme;
    final p = progress;
    return ListView(
      padding: const EdgeInsets.only(bottom: 24),
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(20, 8, 20, 0),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                l.rememberedHeadline(p.rememberedPct),
                key: const Key('remembered-headline'),
                style: text.displaySmall?.copyWith(fontWeight: FontWeight.w600),
              ),
              const SizedBox(height: 4),
              Text(
                l.rememberedCount(p.stats.rememberedCount),
                style: text.titleMedium?.copyWith(color: Brand.slate),
              ),
              const SizedBox(height: 12),
              Wrap(
                spacing: 8,
                children: [
                  Pill(
                    icon: Icons.local_fire_department_outlined,
                    label: l.streakChip(p.stats.streakCurrent),
                  ),
                  Pill(label: l.longestStreak(p.stats.streakLongest)),
                ],
              ),
            ],
          ),
        ),
        SectionTitle(l.weeklyTitle),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 20),
          child: Card(
            child: Padding(
              padding: const EdgeInsets.fromLTRB(16, 20, 16, 12),
              child: WeeklyBars(series: p.series),
            ),
          ),
        ),
        SectionTitle(l.byTopicTitle),
        for (final t in p.byTopic)
          Padding(
            padding: const EdgeInsets.fromLTRB(20, 0, 20, 10),
            child: Card(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Expanded(child: Text(t.name, style: text.titleMedium)),
                        Text('${t.recallPct}%', style: text.titleMedium),
                      ],
                    ),
                    const SizedBox(height: 8),
                    ClipRRect(
                      borderRadius: BorderRadius.circular(4),
                      child: LinearProgressIndicator(
                        value: t.recallPct / 100,
                        minHeight: 6,
                        backgroundColor: Brand.border,
                        color: Brand.ink,
                      ),
                    ),
                    if (t.nextReviewOn != null) ...[
                      const SizedBox(height: 6),
                      Text(
                        l.nextReviewOn(formatIsoDate(context, t.nextReviewOn)),
                        style: text.bodySmall?.copyWith(color: Brand.slate),
                      ),
                    ],
                  ],
                ),
              ),
            ),
          ),
        SectionTitle(l.milestonesTitle),
        for (final m in p.milestones)
          ListTile(
            leading: Icon(
              m.reached ? Icons.check_circle : Icons.radio_button_unchecked,
              color: m.reached ? Brand.mint : Brand.border,
            ),
            title: Text(m.label),
            trailing: Text(
              m.reached ? l.reached : l.notYetReached,
              style: text.bodySmall?.copyWith(color: Brand.slate),
            ),
          ),
      ],
    );
  }
}

/// Eight bars, no axis clutter. The last bar is ink, the rest slate-tinted.
class WeeklyBars extends StatelessWidget {
  const WeeklyBars({required this.series, super.key});
  final List<WeekPoint> series;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    if (series.isEmpty) return const SizedBox(height: 120);
    return SizedBox(
      height: 140,
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.end,
        children: [
          for (final (i, w) in series.indexed) ...[
            if (i > 0) const SizedBox(width: 8),
            Expanded(
              child: Column(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  Text(
                    '${w.rememberedPct}',
                    style: text.labelSmall?.copyWith(color: Brand.slate),
                  ),
                  const SizedBox(height: 4),
                  Container(
                    key: Key('week-bar-$i'),
                    height: 100 * (w.rememberedPct.clamp(0, 100) / 100),
                    decoration: BoxDecoration(
                      color: i == series.length - 1 ? Brand.ink : Brand.border,
                      borderRadius:
                          const BorderRadius.vertical(top: Radius.circular(4)),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ],
      ),
    );
  }
}
