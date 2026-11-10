import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../core/api/models/models.dart';
import '../../../core/api/reelearn_api.dart';
import '../../../core/providers.dart';
import '../../player/data/telemetry.dart';
import '../../player/domain/stack_model.dart';

/// Practice ("keep going" after the minute): ranked batches from `/me/next`
/// appended to one swipe stack, a pause card every `break_every_s` of practice
/// time, and an end card when the company's caps, work hours or the catalogue
/// close the session.
class PracticeState {
  const PracticeState({
    this.sessionId,
    this.cards = const [LoadingStackCard()],
    this.loading = false,
    this.started = false,
    this.error,
    this.autoplayNext = true,
    this.peerSharing = true,
    this.breakEveryS,
  });

  final String? sessionId;

  /// Content cards, then exactly one trailing card: loading or the end.
  final List<StackCard> cards;
  final bool loading;
  final bool started;
  final Object? error;
  final bool autoplayNext;
  final bool peerSharing;
  final int? breakEveryS;

  bool get ended => cards.isNotEmpty && cards.last is PracticeEndStackCard;
  int get units => unitCount(cards);

  PracticeState copyWith({
    String? sessionId,
    List<StackCard>? cards,
    bool? loading,
    bool? started,
    Object? error,
    bool clearError = false,
    bool? autoplayNext,
    bool? peerSharing,
    int? breakEveryS,
  }) =>
      PracticeState(
        sessionId: sessionId ?? this.sessionId,
        cards: cards ?? this.cards,
        loading: loading ?? this.loading,
        started: started ?? this.started,
        error: clearError ? null : (error ?? this.error),
        autoplayNext: autoplayNext ?? this.autoplayNext,
        peerSharing: peerSharing ?? this.peerSharing,
        breakEveryS: breakEveryS ?? this.breakEveryS,
      );
}

class PracticeController extends StateNotifier<PracticeState> {
  PracticeController(this._api, this._telemetry) : super(const PracticeState());

  final ReelearnApi _api;
  final TelemetryService _telemetry;

  /// Load the next batch once the learner is this close to the end.
  static const prefetchDistance = 3;

  int _sinceBreakMs = 0;
  int _breaks = 0;
  int _index = 0;

  Future<void> start() async {
    if (state.started) return;
    state = state.copyWith(started: true);
    await loadMore();
  }

  Future<void> loadMore() async {
    if (state.loading || state.ended) return;
    state = state.copyWith(loading: true, clearError: true);
    try {
      final batch = await _api.getNext(sessionId: state.sessionId);
      final content = state.cards.where(
        (c) => c is! LoadingStackCard && c is! PracticeEndStackCard,
      );
      final added = contentCards(
        [for (final i in batch.items) i.toPlanItem()],
        firstUnit: unitCount(state.cards),
      );
      final trailing = batch.budget.open && batch.items.isNotEmpty
          ? const LoadingStackCard()
          : PracticeEndStackCard(
              batch.budget.stop ?? PracticeStop.exhausted,
            );
      state = state.copyWith(
        sessionId: batch.sessionId,
        cards: List.unmodifiable([...content, ...added, trailing]),
        loading: false,
        autoplayNext: batch.autoplayNext,
        peerSharing: batch.peerSharing,
        breakEveryS: batch.budget.breakEveryS,
      );
    } on Object catch (e) {
      state = state.copyWith(loading: false, error: e);
    }
  }

  /// The stack moved to [index]: prefetch near the end.
  void onIndex(int index) {
    _index = index;
    if (index >= state.cards.length - prefetchDistance) unawaited(loadMore());
  }

  /// Practice time while the screen is visible. Reports it, and inserts a
  /// pause card after the current one when the company's interval has passed.
  Future<void> addActive(int ms) async {
    if (ms <= 0) return;
    await _telemetry.signals(sessionId: state.sessionId, activeMs: ms);
    final every = state.breakEveryS;
    if (every == null || every <= 0) return;
    _sinceBreakMs += ms;
    if (_sinceBreakMs < every * 1000) return;
    _sinceBreakMs = 0;
    final at = (_index + 1).clamp(0, state.cards.length - 1);
    if (state.cards[at] is BreakStackCard ||
        state.ended && at >= state.cards.length - 1) {
      return;
    }
    final cards = [...state.cards]..insert(at, BreakStackCard(++_breaks));
    state = state.copyWith(cards: List.unmodifiable(cards));
  }

  Future<void> signal(EngagementSignal signal) =>
      _telemetry.signals(sessionId: state.sessionId, events: [signal]);

  Future<void> end() async {
    final id = state.sessionId;
    if (id == null) return;
    try {
      await _api.endSession(id);
    } on Object {
      // The server closes idle sessions on its own; nothing to recover.
    }
  }
}

final practiceProvider =
    StateNotifierProvider.autoDispose<PracticeController, PracticeState>(
  (ref) => PracticeController(
    ref.watch(apiProvider),
    ref.watch(telemetryProvider),
  ),
);
