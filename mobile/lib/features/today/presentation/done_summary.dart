import 'package:flutter/material.dart';

import '../../../app/l10n/gen/app_localizations.dart';
import '../../../app/theme.dart';
import '../../../core/api/models/models.dart';

/// Shared by the stack's Done card and the `/today/done` screen.
class DoneSummary extends StatelessWidget {
  const DoneSummary({
    required this.remembered,
    required this.total,
    required this.stats,
    required this.onBackToWork,
    required this.onOpenSaved,
    this.onKeepGoing,
    super.key,
  });

  final int remembered;
  final int total;
  final UserStats stats;
  final VoidCallback onBackToWork;
  final VoidCallback onOpenSaved;

  /// Practice after the minute, when the company offers it.
  final VoidCallback? onKeepGoing;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final text = Theme.of(context).textTheme;
    return ColoredBox(
      color: Brand.paper,
      child: SafeArea(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(24, 48, 24, 24),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Container(
                key: const Key('done-check'),
                width: 64,
                height: 64,
                decoration: const BoxDecoration(
                  color: Brand.mint,
                  shape: BoxShape.circle,
                ),
                child: const Icon(Icons.check, color: Brand.night, size: 36),
              ),
              const SizedBox(height: 24),
              Text(
                l.doneTitle,
                style:
                    text.headlineMedium?.copyWith(fontWeight: FontWeight.w600),
              ),
              const SizedBox(height: 12),
              Text(l.doneRemembered(remembered, total), style: text.titleLarge),
              const SizedBox(height: 8),
              Text(
                l.streakLine(stats.streakCurrent),
                style: text.titleMedium?.copyWith(color: Brand.slate),
              ),
              const SizedBox(height: 8),
              Text(
                l.freezeNote(stats.freezesAvailable),
                style: text.bodyMedium?.copyWith(color: Brand.slate),
              ),
              const Spacer(),
              if (onKeepGoing == null)
                Text(
                  l.nothingMore,
                  style: text.bodyMedium?.copyWith(color: Brand.slate),
                )
              else ...[
                Text(
                  l.keepGoingHint,
                  style: text.bodyMedium?.copyWith(color: Brand.slate),
                ),
                const SizedBox(height: 12),
                OutlinedButton.icon(
                  key: const Key('keep-going'),
                  onPressed: onKeepGoing,
                  icon: const Icon(Icons.play_arrow_rounded),
                  label: Text(l.keepGoing),
                ),
              ],
              const SizedBox(height: 16),
              FilledButton(
                key: const Key('back-to-work'),
                onPressed: onBackToWork,
                child: Text(l.backToWork),
              ),
              const SizedBox(height: 4),
              Center(
                child: TextButton(
                  key: const Key('open-saved'),
                  onPressed: onOpenSaved,
                  child: Text(l.openSaved),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
