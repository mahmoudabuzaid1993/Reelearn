import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../../app/l10n/gen/app_localizations.dart';
import '../../../app/theme.dart';
import '../../../core/api/models/models.dart';
import '../../today/presentation/done_summary.dart';
import '../domain/stack_model.dart';
import 'question_card_view.dart';
import 'reel_card_view.dart';
import 'segments_bar.dart';

/// The swipe stack: a vertical pager with one full-screen card per plan item
/// and a final Done card. Pure: no providers, no HTTP; every effect is a
/// callback. Gesture rules live here; business rules in `stack_model.dart`.
class SwipeStack extends StatefulWidget {
  const SwipeStack({
    required this.cards,
    required this.onPick,
    required this.onClose,
    required this.onBackToWork,
    required this.onOpenSaved,
    this.onArrive,
    this.onWatched,
    this.onToggleSave,
    this.onIndexChanged,
    this.stats = UserStats.empty,
    this.initialIndex = 0,
    this.initialAnswers = const {},
    this.captionsDefault = true,
    this.mediaBuilder,
    this.reducedMotion,
    this.onLeave,
    this.onRewatch,
    this.onNotForMe,
    this.onShare,
    this.onKeepGoing,
    this.extraCardBuilder,
    this.showSegments = true,
    this.autoAdvanceAfterAnswer,
    super.key,
  });

  final List<StackCard> cards;

  /// Returns the outcome, or null when the answer was queued offline.
  final Future<AnswerOutcome?> Function(QuestionStackCard card, String key)
      onPick;
  final VoidCallback onClose;
  final VoidCallback onBackToWork;
  final VoidCallback onOpenSaved;

  /// Fired when a card becomes the active one (watch "arrival" event).
  final void Function(StackCard card, int index)? onArrive;

  /// Fired once per reel when playback passes 90 %.
  final void Function(ReelStackCard card, int watchedMs)? onWatched;

  /// Returns the new saved state.
  final Future<bool> Function(ReelStackCard card)? onToggleSave;
  final void Function(int index)? onIndexChanged;
  final UserStats stats;
  final int initialIndex;
  final Map<String, StackAnswer> initialAnswers;
  final bool captionsDefault;
  final ReelMediaBuilder? mediaBuilder;

  /// Null reads `MediaQuery.disableAnimations`.
  final bool? reducedMotion;

  /// Fired when the learner leaves a reel card, with how far they watched;
  /// under three seconds the screen records a quick skip.
  final void Function(ReelStackCard card, int watchedMs)? onLeave;

  /// Fired when a finished reel starts again from the top.
  final void Function(ReelStackCard card)? onRewatch;

  /// "Not for me": the stack moves past the reel and its question.
  final void Function(ReelStackCard card)? onNotForMe;

  /// "Send to a colleague". Hidden when null.
  final void Function(ReelStackCard card)? onShare;

  /// "Keep going" on the Done card. Hidden when null.
  final VoidCallback? onKeepGoing;

  /// Renders practice-only cards (pause, loading, end).
  final Widget Function(StackCard card)? extraCardBuilder;

  /// Practice runs open-ended, so it hides the per-unit segments.
  final bool showSegments;

  /// Move on by itself this long after an answer is revealed.
  final Duration? autoAdvanceAfterAnswer;

  static const snapDuration = Duration(milliseconds: 260);
  static const flingVelocity = 700.0;
  static const toastDuration = Duration(milliseconds: 1800);

  @override
  State<SwipeStack> createState() => _SwipeStackState();
}

class _SwipeStackState extends State<SwipeStack> with TickerProviderStateMixin {
  late int _index;
  late final Map<String, StackAnswer> _answers;
  double _raw = 0; // accumulated finger travel this gesture (px)
  double _offset = 0; // what the cards actually show
  bool _dragging = false;
  bool _gatedThisDrag = false;

  late final AnimationController _snap;
  Animation<double>? _snapAnim;
  late final AnimationController _toast;

  // Per-reel UI state, keyed by plan item id.
  final Map<String, bool> _captions = {};
  final Map<String, bool> _listenOnly = {};
  final Map<String, bool> _saved = {};
  final Map<String, double> _progress = {};
  final Set<String> _watched90 = {};
  double _height = 0;

  List<StackCard> get _cards => widget.cards;
  StackCard get _current => _cards[_index];

  bool _reduced(BuildContext context) =>
      widget.reducedMotion ?? MediaQuery.disableAnimationsOf(context);

  @override
  void initState() {
    super.initState();
    _index = widget.initialIndex.clamp(0, math.max(0, _cards.length - 1));
    _answers = Map.of(widget.initialAnswers);
    _snap = AnimationController(vsync: this, duration: SwipeStack.snapDuration)
      ..addListener(() {
        final a = _snapAnim;
        if (a != null) setState(() => _offset = a.value);
      });
    _toast =
        AnimationController(vsync: this, duration: SwipeStack.toastDuration);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) widget.onArrive?.call(_current, _index);
    });
  }

  @override
  void dispose() {
    _snap.dispose();
    _toast.dispose();
    super.dispose();
  }

  // ----------------------------------------------------------- gating

  bool get _canForward =>
      canGoForward(cards: _cards, index: _index, answers: _answers);
  bool get _canBackward => canGoBackward(_index);
  bool get _isGated =>
      isGatedByQuestion(cards: _cards, index: _index, answers: _answers);

  // ----------------------------------------------------------- gestures

  static double _rubber(double x) => 90 * (1 - math.exp(-x.abs() / 180));

  void _onDragStart(DragStartDetails d) {
    if (_snap.isAnimating) _snap.stop();
    _dragging = true;
    _raw = 0;
    _gatedThisDrag = false;
  }

  void _onDragUpdate(DragUpdateDetails d) {
    if (!_dragging) return;
    _raw += d.delta.dy;
    double next;
    if (_raw < 0) {
      // Finger moving up: forward.
      if (_canForward) {
        next = _raw;
      } else {
        next = -_rubber(_raw);
        if (_isGated && _raw < -24) _gatedThisDrag = true;
      }
    } else {
      next = _canBackward ? _raw : _rubber(_raw);
    }
    setState(() => _offset = next);
  }

  void _onDragEnd(DragEndDetails d, double height) {
    if (!_dragging) return;
    _dragging = false;
    final v = d.primaryVelocity ?? 0;
    final threshold = height / 3;
    final wantsForward = _raw < -threshold || v < -SwipeStack.flingVelocity;
    final wantsBackward = _raw > threshold || v > SwipeStack.flingVelocity;

    if (wantsForward && _canForward) {
      _go(_index + 1, height);
      return;
    }
    if (wantsBackward && _canBackward) {
      _go(_index - 1, height);
      return;
    }
    final triedForward =
        wantsForward || _gatedThisDrag || v < -SwipeStack.flingVelocity;
    if (triedForward && _isGated) _showToast();
    _snapTo(0);
  }

  void _go(int target, double height) {
    final leaving = _current;
    if (leaving is ReelStackCard && target != _index) {
      final ms =
          ((_progress[leaving.item.id] ?? 0) * leaving.reel.durationMs).round();
      widget.onLeave?.call(leaving, ms);
    }
    final dir = target > _index ? -1 : 1;
    if (_reduced(context)) {
      setState(() {
        _index = target;
        _offset = 0;
        _raw = 0;
      });
      _arrived();
      return;
    }
    _snapAnim = Tween<double>(begin: _offset, end: dir * height).animate(
      CurvedAnimation(parent: _snap, curve: Curves.easeOutCubic),
    );
    _snap.forward(from: 0).whenComplete(() {
      if (!mounted) return;
      setState(() {
        _index = target;
        _offset = 0;
        _raw = 0;
      });
      _arrived();
    });
  }

  void _snapTo(double value) {
    if (_reduced(context)) {
      setState(() {
        _offset = value;
        _raw = 0;
      });
      return;
    }
    _snapAnim = Tween<double>(begin: _offset, end: value).animate(
      CurvedAnimation(parent: _snap, curve: Curves.easeOutCubic),
    );
    _snap.forward(from: 0).whenComplete(() {
      if (mounted) _raw = 0;
    });
  }

  void _arrived() {
    widget.onIndexChanged?.call(_index);
    widget.onArrive?.call(_current, _index);
  }

  void _showToast() => _toast.forward(from: 0);

  // ----------------------------------------------------------- actions

  Future<void> _pick(QuestionStackCard card, String key) async {
    final id = card.item.id;
    if (_answers.containsKey(id)) return; // pick once
    setState(() => _answers[id] = StackAnswer(chosenKey: key));
    final outcome = await widget.onPick(card, key);
    if (!mounted) return;
    setState(() {
      _answers[id] = StackAnswer(
        chosenKey: key,
        outcome: outcome,
        pendingOffline: outcome == null,
      );
    });
    final wait = widget.autoAdvanceAfterAnswer;
    if (wait != null) {
      await Future<void>.delayed(wait);
      if (!mounted) return;
      final current = _current;
      if (current is QuestionStackCard &&
          current.item.id == id &&
          _canForward &&
          _height > 0) {
        _go(_index + 1, _height);
      }
    }
  }

  void _notForMe(ReelStackCard card) {
    widget.onNotForMe?.call(card);
    final next = nextUnitStart(_cards, _index);
    if (next != null && _height > 0) _go(next, _height);
  }

  void _onProgress(ReelStackCard card, int positionMs, int durationMs) {
    if (durationMs <= 0) return;
    final p = positionMs / durationMs;
    final before = _progress[card.item.id] ?? 0;
    if (_watched90.contains(card.item.id) && before >= 0.9 && p < 0.05) {
      widget.onRewatch?.call(card);
    }
    if (before != p) {
      setState(() => _progress[card.item.id] = p);
    }
    if (p >= 0.9 && !_watched90.contains(card.item.id)) {
      _watched90.add(card.item.id);
      widget.onWatched?.call(card, positionMs);
    }
  }

  Future<void> _toggleSave(ReelStackCard card) async {
    final id = card.item.id;
    final before = _saved[id] ?? card.reel.saved;
    setState(() => _saved[id] = !before); // optimistic
    final cb = widget.onToggleSave;
    if (cb == null) return;
    final after = await cb(card);
    if (mounted) setState(() => _saved[id] = after);
  }

  // ----------------------------------------------------------- build

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final segments = computeSegments(
      cards: _cards,
      answers: _answers,
      currentIndex: _index,
    );

    return LayoutBuilder(
      builder: (context, constraints) {
        final h = constraints.maxHeight;
        _height = h;
        return GestureDetector(
          key: const Key('swipe-stack'),
          behavior: HitTestBehavior.opaque,
          onVerticalDragStart: _onDragStart,
          onVerticalDragUpdate: _onDragUpdate,
          onVerticalDragEnd: (d) => _onDragEnd(d, h),
          child: ColoredBox(
            color: Brand.night,
            child: Stack(
              fit: StackFit.expand,
              children: [
                for (var i = _index - 1; i <= _index + 1; i++)
                  if (i >= 0 && i < _cards.length)
                    Positioned.fill(
                      key: Key('stack-card-${_cards[i].key}'),
                      child: Transform.translate(
                        offset: Offset(0, (i - _index) * h + _offset),
                        child: _buildCard(_cards[i], isActive: i == _index),
                      ),
                    ),
                // Top: segments + close.
                Positioned(
                  left: 0,
                  right: 0,
                  top: 0,
                  child: SafeArea(
                    bottom: false,
                    child: Padding(
                      padding:
                          const EdgeInsetsDirectional.fromSTEB(16, 8, 4, 0),
                      child: Row(
                        children: [
                          Expanded(
                            child: _current is DoneStackCard ||
                                    !widget.showSegments
                                ? const SizedBox.shrink()
                                : SegmentsBar(states: segments),
                          ),
                          IconButton(
                            key: const Key('stack-close'),
                            tooltip: l.close,
                            onPressed: widget.onClose,
                            iconSize: 24,
                            constraints: const BoxConstraints.tightFor(
                              width: Brand.target,
                              height: Brand.target,
                            ),
                            icon: Icon(
                              Icons.close,
                              color: _current is ReelStackCard
                                  ? Brand.white
                                  : Brand.ink,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
                // Toast.
                Positioned(
                  left: 24,
                  right: 24,
                  bottom: 120,
                  child: IgnorePointer(
                    child: AnimatedBuilder(
                      animation: _toast,
                      builder: (context, _) {
                        final v = _toast.value;
                        final visible = _toast.isAnimating;
                        final reduced = _reduced(context);
                        final opacity = !visible
                            ? 0.0
                            : reduced
                                ? 1.0
                                : v < 0.75
                                    ? 1.0
                                    : (1 - v) / 0.25;
                        if (opacity == 0) return const SizedBox.shrink();
                        return Opacity(
                          opacity: opacity,
                          child: Center(
                            child: Container(
                              key: const Key('stack-toast'),
                              padding: const EdgeInsets.symmetric(
                                horizontal: 16,
                                vertical: 12,
                              ),
                              decoration: BoxDecoration(
                                color: Brand.ink,
                                borderRadius:
                                    BorderRadius.circular(Brand.radius),
                              ),
                              child: Text(
                                l.answerFirstToast,
                                style: Theme.of(context)
                                    .textTheme
                                    .bodyMedium
                                    ?.copyWith(color: Brand.white),
                              ),
                            ),
                          ),
                        );
                      },
                    ),
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }

  Widget _buildCard(StackCard card, {required bool isActive}) {
    switch (card) {
      case ReelStackCard():
        final id = card.item.id;
        return ReelCardView(
          card: card,
          isActive: isActive,
          captionsOn: _captions[id] ??
              (widget.captionsDefault && card.reel.captionsOnByDefault),
          listenOnly: _listenOnly[id] ?? false,
          saved: _saved[id] ?? card.reel.saved,
          progress: _progress[id] ?? 0,
          onToggleCaptions: () => setState(
            () => _captions[id] = !(_captions[id] ??
                (widget.captionsDefault && card.reel.captionsOnByDefault)),
          ),
          onToggleListenOnly: () =>
              setState(() => _listenOnly[id] = !(_listenOnly[id] ?? false)),
          onToggleSave: () => _toggleSave(card),
          onProgress: (p, d) => _onProgress(card, p, d),
          mediaBuilder: widget.mediaBuilder,
          onNotForMe: widget.onNotForMe == null ? null : () => _notForMe(card),
          onShare: widget.onShare == null ? null : () => widget.onShare!(card),
        );
      case QuestionStackCard():
        return QuestionCardView(
          card: card,
          answer: _answers[card.item.id],
          onPick: (key) => _pick(card, key),
        );
      case DoneStackCard():
        final tally = rememberedTally(_cards, _answers);
        final latestStats = _answers.values
                .map((a) => a.outcome?.stats)
                .whereType<UserStats>()
                .lastOrNull ??
            widget.stats;
        return DoneSummary(
          remembered: tally.remembered,
          total: tally.total,
          stats: latestStats,
          onBackToWork: widget.onBackToWork,
          onOpenSaved: widget.onOpenSaved,
          onKeepGoing: widget.onKeepGoing,
        );
      case BreakStackCard() || LoadingStackCard() || PracticeEndStackCard():
        return widget.extraCardBuilder?.call(card) ??
            const ColoredBox(color: Brand.paper);
    }
  }
}
