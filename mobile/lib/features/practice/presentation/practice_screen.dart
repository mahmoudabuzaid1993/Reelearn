import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../../app/l10n/gen/app_localizations.dart';
import '../../../app/router.dart';
import '../../../app/theme.dart';
import '../../../app/widgets.dart';
import '../../../core/api/models/models.dart';
import '../../../core/auth/session.dart';
import '../../../core/providers.dart';
import '../../player/data/telemetry.dart';
import '../../player/domain/stack_model.dart';
import '../../player/presentation/reel_card_view.dart';
import '../../player/presentation/swipe_stack.dart';
import '../../player/presentation/video_reel_media.dart';
import '../../today/data/today_repository.dart';
import '../application/practice_controller.dart';
import 'share_sheet.dart';

/// "Keep going": ranked practice after the daily minute, in the same swipe
/// stack. Practice time is reported every [heartbeat] while the screen is in
/// the foreground; the company's caps end the session with an end card.
class PracticeScreen extends ConsumerStatefulWidget {
  const PracticeScreen({super.key, this.mediaBuilder});

  /// Tests pass the placeholder; the app plays video.
  final ReelMediaBuilder? mediaBuilder;

  static const heartbeat = Duration(seconds: 30);

  @override
  ConsumerState<PracticeScreen> createState() => _PracticeScreenState();
}

class _PracticeScreenState extends ConsumerState<PracticeScreen>
    with WidgetsBindingObserver {
  final Stopwatch _active = Stopwatch();
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _active.start();
    _timer = Timer.periodic(PracticeScreen.heartbeat, (_) => _flush());
    Future.microtask(() => ref.read(practiceProvider.notifier).start());
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState s) {
    if (s == AppLifecycleState.resumed) {
      _active.start();
    } else {
      _flush();
      _active.stop();
    }
  }

  void _flush() {
    final ms = _active.elapsedMilliseconds;
    if (ms <= 0) return;
    _active.reset();
    if (_active.isRunning ||
        WidgetsBinding.instance.lifecycleState == AppLifecycleState.resumed) {
      _active.start();
    }
    unawaited(ref.read(practiceProvider.notifier).addActive(ms));
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _timer?.cancel();
    super.dispose();
  }

  void _leave(String to) {
    _flush();
    _active.stop();
    final controller = ref.read(practiceProvider.notifier);
    unawaited(controller.end());
    ref.invalidate(todayProvider);
    context.go(to);
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final state = ref.watch(practiceProvider);
    final controller = ref.read(practiceProvider.notifier);
    final telemetry = ref.read(telemetryProvider);
    final me = ref.watch(meProvider);

    if (state.error != null && state.units == 0) {
      return Scaffold(
        backgroundColor: Brand.night,
        body: SafeArea(
          child: Center(
            child: ErrorLine(error: state.error!, onRetry: controller.loadMore),
          ),
        ),
      );
    }

    return Scaffold(
      backgroundColor: Brand.night,
      body: SwipeStack(
        key: const Key('practice-stack'),
        cards: state.cards,
        showSegments: false,
        captionsDefault: me?.captionsOn ?? true,
        mediaBuilder: widget.mediaBuilder ?? videoReelMedia,
        autoAdvanceAfterAnswer:
            state.autoplayNext ? const Duration(milliseconds: 1600) : null,
        onIndexChanged: controller.onIndex,
        onArrive: (card, _) {
          if (card is ReelStackCard) {
            telemetry.recordWatch(
              reelId: card.reel.id,
              watchedMs: 0,
              completed: false,
            );
          }
        },
        onWatched: (card, ms) => telemetry.recordWatch(
          reelId: card.reel.id,
          watchedMs: ms,
          completed: true,
        ),
        onLeave: (card, ms) {
          if (ms < skipThresholdMs) {
            controller.signal(
              EngagementSignal(
                reelId: card.reel.id,
                kind: EngagementKind.skip,
                watchedMs: ms,
                position: card.item.position,
              ),
            );
          }
        },
        onRewatch: (card) => controller.signal(
          EngagementSignal(
            reelId: card.reel.id,
            kind: EngagementKind.rewatch,
            watchedMs: card.reel.durationMs,
          ),
        ),
        onNotForMe: (card) {
          controller.signal(
            EngagementSignal(
              reelId: card.reel.id,
              kind: EngagementKind.notRelevant,
              position: card.item.position,
            ),
          );
          ScaffoldMessenger.maybeOf(context)
            ?..hideCurrentSnackBar()
            ..showSnackBar(SnackBar(content: Text(l.notForMeDone)));
        },
        onShare: state.peerSharing
            ? (card) => showShareSheet(context, ref, card.reel)
            : null,
        onPick: (card, key) => telemetry.answer(
          questionId: card.question.id,
          chosenKey: key,
        ),
        onToggleSave: (card) async {
          final api = ref.read(apiProvider);
          final was = card.reel.saved;
          try {
            if (was) {
              await api.unsaveReel(card.reel.id);
            } else {
              await api.saveReel(card.reel.id);
            }
            return !was;
          } on Object {
            return was;
          }
        },
        extraCardBuilder: (card) => switch (card) {
          BreakStackCard() => _BreakCard(onStop: () => _leave(Routes.today)),
          PracticeEndStackCard(:final stop) =>
            _EndCard(stop: stop, onBack: () => _leave(Routes.today)),
          _ => _LoadingCard(error: state.error, onRetry: controller.loadMore),
        },
        onClose: () => _leave(Routes.today),
        onBackToWork: () => _leave(Routes.today),
        onOpenSaved: () => _leave(Routes.saved),
      ),
    );
  }
}

class _Panel extends StatelessWidget {
  const _Panel({required this.children, super.key});

  final List<Widget> children;

  @override
  Widget build(BuildContext context) => ColoredBox(
        color: Brand.paper,
        child: SafeArea(
          child: Padding(
            padding: const EdgeInsets.fromLTRB(24, 72, 24, 32),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: children,
            ),
          ),
        ),
      );
}

class _BreakCard extends StatelessWidget {
  const _BreakCard({required this.onStop});

  final VoidCallback onStop;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final text = Theme.of(context).textTheme;
    return _Panel(
      key: const Key('practice-break'),
      children: [
        const Icon(Icons.self_improvement, size: 48, color: Brand.mint),
        const SizedBox(height: 20),
        Text(
          l.breakTitle,
          style: text.headlineSmall?.copyWith(fontWeight: FontWeight.w600),
        ),
        const SizedBox(height: 12),
        Text(l.breakBody, style: text.bodyLarge),
        const Spacer(),
        Row(
          children: [
            const Icon(Icons.keyboard_arrow_up, color: Brand.slate),
            const SizedBox(width: 4),
            Text(
              l.breakContinue,
              style: text.labelLarge?.copyWith(color: Brand.slate),
            ),
          ],
        ),
        const SizedBox(height: 12),
        OutlinedButton(
          key: const Key('break-stop'),
          onPressed: onStop,
          child: Text(l.breakStop),
        ),
      ],
    );
  }
}

class _LoadingCard extends StatelessWidget {
  const _LoadingCard({required this.onRetry, this.error});

  final Object? error;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return _Panel(
      key: const Key('practice-loading'),
      children: [
        const Spacer(),
        Center(
          child: error == null
              ? Column(
                  children: [
                    const CircularProgressIndicator(color: Brand.ink),
                    const SizedBox(height: 16),
                    Text(l.practiceLoading),
                  ],
                )
              : Column(
                  children: [
                    Text(l.practiceError),
                    const SizedBox(height: 12),
                    OutlinedButton(onPressed: onRetry, child: Text(l.retry)),
                  ],
                ),
        ),
        const Spacer(),
      ],
    );
  }
}

class _EndCard extends StatelessWidget {
  const _EndCard({required this.stop, required this.onBack});

  final PracticeStop? stop;
  final VoidCallback onBack;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final text = Theme.of(context).textTheme;
    final body = switch (stop) {
      PracticeStop.dailyCap => l.practiceEndDailyCap,
      PracticeStop.sessionCap => l.practiceEndSessionCap,
      PracticeStop.outsideWorkHours => l.practiceEndOutsideHours,
      PracticeStop.paused => l.practiceEndPaused,
      PracticeStop.disabled => l.practiceEndDisabled,
      _ => l.practiceEndExhausted,
    };
    return _Panel(
      key: const Key('practice-end'),
      children: [
        Container(
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
          l.practiceEndTitle,
          style: text.headlineMedium?.copyWith(fontWeight: FontWeight.w600),
        ),
        const SizedBox(height: 12),
        Text(body, style: text.titleMedium?.copyWith(color: Brand.slate)),
        const Spacer(),
        FilledButton(
          key: const Key('practice-back'),
          onPressed: onBack,
          child: Text(l.practiceBackToToday),
        ),
      ],
    );
  }
}
