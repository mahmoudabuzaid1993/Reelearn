import 'package:flutter/material.dart';

import '../../../app/l10n/gen/app_localizations.dart';
import '../../../app/theme.dart';
import '../../../core/api/models/models.dart';
import '../domain/stack_model.dart';

/// What the stack hands to whoever renders the video.
class ReelMediaRequest {
  const ReelMediaRequest({
    required this.reel,
    required this.isActive,
    required this.captionsOn,
    required this.listenOnly,
    required this.onProgress,
  });

  final ReelPlayable reel;
  final bool isActive;
  final bool captionsOn;
  final bool listenOnly;

  /// `(positionMs, durationMs)`; the stack derives the 90 % event from it.
  final void Function(int positionMs, int durationMs) onProgress;
}

typedef ReelMediaBuilder = Widget Function(BuildContext, ReelMediaRequest);

/// Night-coloured placeholder used in tests and when no media is wired.
Widget placeholderReelMedia(BuildContext context, ReelMediaRequest req) =>
    ColoredBox(
      color: Brand.night,
      child: Center(
        child: Icon(
          req.listenOnly ? Icons.graphic_eq : Icons.play_circle_outline,
          color: Brand.white.withValues(alpha: 0.4),
          size: 64,
        ),
      ),
    );

/// A reel card: video, caption line, side actions, source, progress, hint.
/// Pure: everything comes in through the constructor.
class ReelCardView extends StatelessWidget {
  const ReelCardView({
    required this.card,
    required this.isActive,
    required this.captionsOn,
    required this.listenOnly,
    required this.saved,
    required this.progress,
    required this.onToggleCaptions,
    required this.onToggleListenOnly,
    required this.onToggleSave,
    required this.onProgress,
    this.mediaBuilder,
    this.onNotForMe,
    this.onShare,
    super.key,
  });

  final ReelStackCard card;
  final bool isActive;
  final bool captionsOn;
  final bool listenOnly;
  final bool saved;

  /// 0..1 of the video.
  final double progress;
  final VoidCallback onToggleCaptions;
  final VoidCallback onToggleListenOnly;
  final VoidCallback onToggleSave;
  final void Function(int positionMs, int durationMs) onProgress;
  final ReelMediaBuilder? mediaBuilder;

  /// "Not for me" (practice and plan); hidden when null.
  final VoidCallback? onNotForMe;

  /// "Send to a colleague"; hidden when null.
  final VoidCallback? onShare;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final reel = card.reel;
    final text = Theme.of(context).textTheme;
    final media = (mediaBuilder ?? placeholderReelMedia)(
      context,
      ReelMediaRequest(
        reel: reel,
        isActive: isActive,
        captionsOn: captionsOn,
        listenOnly: listenOnly,
        onProgress: onProgress,
      ),
    );

    return ColoredBox(
      color: Brand.night,
      child: Stack(
        fit: StackFit.expand,
        children: [
          media,
          // Bottom text block.
          PositionedDirectional(
            start: 20,
            end: 84,
            bottom: 72,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                if (card.item.practiceReason != null) ...[
                  _ReasonChip(
                    text: reasonLine(
                      l,
                      card.item.practiceReason!,
                      card.item.reasonDetail ?? reel.topicName,
                    ),
                  ),
                  const SizedBox(height: 8),
                ],
                Text(
                  reel.topicName.toUpperCase(),
                  style: text.labelMedium?.copyWith(
                    color: Brand.white.withValues(alpha: 0.7),
                    letterSpacing: 1.2,
                  ),
                ),
                const SizedBox(height: 6),
                Text(
                  reel.title,
                  style: text.titleLarge?.copyWith(
                    color: Brand.white,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                if (reel.presenterName != null) ...[
                  const SizedBox(height: 4),
                  Text(
                    l.presenterLine(reel.presenterName!),
                    style: text.bodyMedium?.copyWith(
                      color: Brand.white.withValues(alpha: 0.8),
                    ),
                  ),
                ],
                if (captionsOn && reel.captionText != null) ...[
                  const SizedBox(height: 12),
                  Container(
                    key: const Key('reel-caption'),
                    padding: const EdgeInsets.symmetric(
                      horizontal: 10,
                      vertical: 6,
                    ),
                    decoration: BoxDecoration(
                      color: Brand.night.withValues(alpha: 0.85),
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Text(
                      reel.captionText!,
                      style: text.bodyLarge?.copyWith(color: Brand.white),
                    ),
                  ),
                ],
                if (reel.sourceCitation != null) ...[
                  const SizedBox(height: 10),
                  Text(
                    l.sourceLine(reel.sourceCitation!),
                    style: text.bodySmall?.copyWith(
                      color: Brand.white.withValues(alpha: 0.6),
                    ),
                  ),
                ],
              ],
            ),
          ),
          // Side actions (end edge; mirrors under RTL).
          PositionedDirectional(
            end: 12,
            bottom: 96,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                _SideAction(
                  key: const Key('action-listen-only'),
                  icon:
                      listenOnly ? Icons.headphones : Icons.headphones_outlined,
                  label: l.listenOnly,
                  active: listenOnly,
                  onTap: onToggleListenOnly,
                ),
                _SideAction(
                  key: const Key('action-save'),
                  icon: saved ? Icons.bookmark : Icons.bookmark_outline,
                  label: saved ? l.savedReel : l.saveReel,
                  active: saved,
                  onTap: onToggleSave,
                ),
                _SideAction(
                  key: const Key('action-captions'),
                  icon: captionsOn
                      ? Icons.closed_caption
                      : Icons.closed_caption_off_outlined,
                  label: l.captions,
                  active: captionsOn,
                  onTap: onToggleCaptions,
                ),
                if (onShare != null)
                  _SideAction(
                    key: const Key('action-share'),
                    icon: Icons.send_outlined,
                    label: l.sendReel,
                    active: false,
                    onTap: onShare!,
                  ),
                if (onNotForMe != null)
                  _SideAction(
                    key: const Key('action-not-for-me'),
                    icon: Icons.do_not_disturb_on_outlined,
                    label: l.notForMe,
                    active: false,
                    onTap: onNotForMe!,
                  ),
                _SideAction(
                  key: const Key('action-why'),
                  icon: Icons.info_outline,
                  label: l.whyThis,
                  active: false,
                  onTap: () => _showWhy(context, reel),
                ),
              ],
            ),
          ),
          // Hint + progress.
          Positioned(
            left: 0,
            right: 0,
            bottom: 0,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    Icon(
                      Icons.keyboard_arrow_up,
                      color: Brand.white.withValues(alpha: 0.8),
                      size: 20,
                    ),
                    const SizedBox(width: 4),
                    Text(
                      card.hasQuestion ? l.swipeHint : l.swipeHintNext,
                      style: text.labelLarge?.copyWith(
                        color: Brand.white.withValues(alpha: 0.8),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                SizedBox(
                  height: 3,
                  child: LinearProgressIndicator(
                    key: const Key('reel-progress'),
                    value: progress.clamp(0, 1),
                    backgroundColor: Brand.white.withValues(alpha: 0.2),
                    color: Brand.white,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  void _showWhy(BuildContext context, ReelPlayable reel) {
    final l = AppLocalizations.of(context);
    showModalBottomSheet<void>(
      context: context,
      backgroundColor: Brand.white,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(Brand.radius)),
      ),
      builder: (ctx) => SafeArea(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(20, 20, 20, 24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(l.whyThis, style: Theme.of(ctx).textTheme.titleMedium),
              const SizedBox(height: 12),
              Text(reel.whyAmISeeingThis),
              if (reel.sourceCitation != null) ...[
                const SizedBox(height: 8),
                Text(
                  l.sourceLine(reel.sourceCitation!),
                  style: Theme.of(ctx)
                      .textTheme
                      .bodySmall
                      ?.copyWith(color: Brand.slate),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

/// "Sent by Sara", "From Customer service", "Popular in Jeddah"...
String reasonLine(AppLocalizations l, String reason, String detail) =>
    switch (reason) {
      'shared' => l.reasonShared(detail),
      'mandatory' => l.reasonMandatory(detail),
      'followed' => l.reasonFollowed(detail),
      'interest' => l.reasonInterest(detail),
      'popular' => l.reasonPopular(detail),
      'new' => l.reasonNew(detail),
      'explore' => l.reasonExplore,
      'review' => l.reasonReview,
      _ => l.reasonFrom(detail),
    };

class _ReasonChip extends StatelessWidget {
  const _ReasonChip({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) => Container(
        key: const Key('reel-reason'),
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
        decoration: BoxDecoration(
          color: Brand.white.withValues(alpha: 0.16),
          borderRadius: BorderRadius.circular(999),
        ),
        child: Text(
          text,
          style: Theme.of(context)
              .textTheme
              .labelMedium
              ?.copyWith(color: Brand.white),
        ),
      );
}

class _SideAction extends StatelessWidget {
  const _SideAction({
    required this.icon,
    required this.label,
    required this.active,
    required this.onTap,
    super.key,
  });

  final IconData icon;
  final String label;
  final bool active;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) => Semantics(
        button: true,
        label: label,
        selected: active,
        child: Material(
          type: MaterialType.transparency,
          child: InkWell(
            onTap: onTap,
            borderRadius: BorderRadius.circular(Brand.radius),
            child: SizedBox(
              width: 60,
              height: 60,
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(icon, color: Brand.white, size: 26),
                  const SizedBox(height: 2),
                  Text(
                    label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context)
                        .textTheme
                        .labelSmall
                        ?.copyWith(color: Brand.white),
                  ),
                ],
              ),
            ),
          ),
        ),
      );
}
