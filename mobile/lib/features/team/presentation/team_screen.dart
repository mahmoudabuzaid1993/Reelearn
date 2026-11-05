import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../../app/l10n/gen/app_localizations.dart';
import '../../../app/router.dart';
import '../../../app/theme.dart';
import '../../../app/widgets.dart';
import '../../../core/api/models/models.dart';
import '../data/team_repository.dart';

class TeamScreen extends ConsumerWidget {
  const TeamScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final team = ref.watch(teamProvider);

    return Scaffold(
      appBar: AppBar(
        title: Text(l.teamTitle),
        actions: [
          TextButton.icon(
            key: const Key('record-reel'),
            onPressed: () => context.push(Routes.record),
            icon: const Icon(Icons.videocam_outlined, size: 18),
            label: Text(l.recordReel),
          ),
        ],
      ),
      body: team.when(
        loading: () => const SkeletonList(),
        error: (e, _) =>
            ErrorLine(error: e, onRetry: () => ref.invalidate(teamProvider)),
        data: (t) => TeamBody(
          team: t,
          onNudge: (m) async {
            final result =
                await ref.read(teamProvider.notifier).nudge(m.userId);
            if (!context.mounted) return;
            final msg = result == null
                ? l.errorGeneric
                : result.queued > 0
                    ? l.nudgeSent(m.fullName)
                    : l.nudgeSkipped(m.fullName);
            ScaffoldMessenger.of(context)
              ..hideCurrentSnackBar()
              ..showSnackBar(SnackBar(content: Text(msg)));
          },
        ),
      ),
    );
  }
}

/// Pure body: readiness per member per topic, team recall, nudge buttons.
class TeamBody extends StatelessWidget {
  const TeamBody({required this.team, required this.onNudge, super.key});
  final TeamReadiness team;
  final void Function(TeamMember member) onNudge;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final text = Theme.of(context).textTheme;
    return ListView(
      padding: const EdgeInsets.only(bottom: 24),
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(20, 8, 20, 0),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                l.teamRecall(team.teamRecallPct),
                key: const Key('team-recall'),
                style:
                    text.headlineSmall?.copyWith(fontWeight: FontWeight.w600),
              ),
              const SizedBox(height: 4),
              Text(
                l.activeThisWeek(team.activeThisWeek, team.teamSize),
                style: text.bodyMedium?.copyWith(color: Brand.slate),
              ),
              const SizedBox(height: 12),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  for (final t in team.topics)
                    Pill(label: '${t.name} · ${t.readyCount}/${team.teamSize}'),
                ],
              ),
            ],
          ),
        ),
        const SizedBox(height: 12),
        for (final m in team.members)
          Padding(
            padding: const EdgeInsets.fromLTRB(20, 0, 20, 10),
            child: MemberCard(
              member: m,
              topics: team.topics,
              onNudge: () => onNudge(m),
            ),
          ),
        Padding(
          padding: const EdgeInsets.fromLTRB(20, 8, 20, 0),
          child: Text(
            l.privacyFootnote,
            style: text.bodySmall?.copyWith(color: Brand.slate),
          ),
        ),
      ],
    );
  }
}

class MemberCard extends StatelessWidget {
  const MemberCard({
    required this.member,
    required this.topics,
    required this.onNudge,
    super.key,
  });

  final TeamMember member;
  final List<TeamTopic> topics;
  final VoidCallback onNudge;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final text = Theme.of(context).textTheme;
    return Card(
      key: Key('member-${member.userId}'),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    member.fullName,
                    style:
                        text.titleMedium?.copyWith(fontWeight: FontWeight.w600),
                  ),
                ),
                Text(
                  member.language.toUpperCase(),
                  style: text.labelSmall?.copyWith(color: Brand.slate),
                ),
              ],
            ),
            const SizedBox(height: 10),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                for (final t in topics)
                  ReadinessChip(
                    key: Key('readiness-${member.userId}-${t.id}'),
                    topic: t.name,
                    state: member.stateFor(t.id),
                  ),
              ],
            ),
            const SizedBox(height: 12),
            Align(
              alignment: AlignmentDirectional.centerEnd,
              child: member.nudgedToday
                  ? Pill(
                      key: Key('nudged-${member.userId}'),
                      icon: Icons.check,
                      label: l.nudgedToday,
                    )
                  : OutlinedButton.icon(
                      key: Key('nudge-${member.userId}'),
                      onPressed: onNudge,
                      icon: const Icon(Icons.notifications_outlined, size: 18),
                      label: Text(l.nudge),
                    ),
            ),
          ],
        ),
      ),
    );
  }
}

class ReadinessChip extends StatelessWidget {
  const ReadinessChip({required this.topic, required this.state, super.key});
  final String topic;
  final ReadinessState state;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final (label, color, fg) = switch (state) {
      ReadinessState.ready => (l.ready, Brand.mint, Brand.night),
      ReadinessState.notYet => (l.notYet, Brand.white, Brand.ink),
      ReadinessState.notStarted => (l.notStarted, Brand.paper, Brand.slate),
    };
    return Semantics(
      label: '$topic: $label',
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          color: color,
          borderRadius: BorderRadius.circular(999),
          border: Border.all(
            color: state == ReadinessState.ready ? Brand.mint : Brand.border,
          ),
        ),
        child: Text(
          '$topic · $label',
          style: Theme.of(context).textTheme.labelMedium?.copyWith(color: fg),
        ),
      ),
    );
  }
}
