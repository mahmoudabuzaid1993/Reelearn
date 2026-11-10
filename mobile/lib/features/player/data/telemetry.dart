import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:uuid/uuid.dart';

import '../../../core/api/models/models.dart';
import '../../../core/api/reelearn_api.dart';
import '../../../core/errors/app_error.dart';
import '../../../core/offline/connectivity.dart';
import '../../../core/offline/offline_queue.dart';
import '../../../core/providers.dart';

/// Sends a queued event to the API with its persisted idempotency key.
Future<void> sendQueuedEvent(ReelearnApi api, QueuedEvent event) async {
  switch (event.kind) {
    case QueuedKind.watch:
      await api.postWatch(
        WatchEvent.fromJson(event.body),
        idempotencyKey: event.id,
      );
    case QueuedKind.answer:
      await api.postAnswer(
        AnswerEvent.fromJson(event.body),
        idempotencyKey: event.id,
      );
    case QueuedKind.events:
      await api.postEvents(
        EngagementBatch.fromJson(event.body),
        idempotencyKey: event.id,
      );
  }
}

/// Watch and answer events, with offline persistence and replay.
///
/// * `recordWatch` always goes through the queue (fire and forget).
/// * `answer` is sent directly because the card needs the outcome; if the
///   network is down the event is queued with the same key and the card shows
///   "answer saved" instead of a reveal.
class TelemetryService {
  TelemetryService({
    required ReelearnApi api,
    required OfflineQueue queue,
    required ConnectivityWatcher connectivity,
    Uuid? uuid,
    DateTime Function()? clock,
  })  : _api = api,
        _queue = queue,
        _connectivity = connectivity,
        _uuid = uuid ?? const Uuid(),
        _clock = clock ?? DateTime.now;

  final ReelearnApi _api;
  final OfflineQueue _queue;
  final ConnectivityWatcher _connectivity;
  final Uuid _uuid;
  final DateTime Function() _clock;
  StreamSubscription<bool>? _sub;

  OfflineQueue get queue => _queue;

  Future<void> start() async {
    await _queue.load();
    _sub ??= _connectivity.online.listen((online) {
      if (online) unawaited(_queue.replay());
    });
    if (await _connectivity.isOnline()) unawaited(_queue.replay());
  }

  Future<void> recordWatch({
    required String reelId,
    required int watchedMs,
    required bool completed,
    String? planItemId,
  }) async {
    final event = QueuedEvent(
      id: _uuid.v4(),
      kind: QueuedKind.watch,
      body: WatchEvent(
        planItemId: planItemId,
        reelId: reelId,
        watchedMs: watchedMs,
        completed: completed,
      ).toJson(),
      createdAt: _clock().millisecondsSinceEpoch,
    );
    await _queue.enqueue(event);
    unawaited(_queue.replay());
  }

  /// Engagement signals (start, quick skip, rewatch, not for me) and practice
  /// time, through the same offline queue as watches.
  Future<void> signals({
    List<EngagementSignal> events = const [],
    String? sessionId,
    int? activeMs,
  }) async {
    if (events.isEmpty && (activeMs ?? 0) <= 0) return;
    final event = QueuedEvent(
      id: _uuid.v4(),
      kind: QueuedKind.events,
      body: EngagementBatch(
        sessionId: sessionId,
        activeMs: activeMs,
        events: events,
      ).toJson(),
      createdAt: _clock().millisecondsSinceEpoch,
    );
    await _queue.enqueue(event);
    unawaited(_queue.replay());
  }

  Future<AnswerOutcome?> answer({
    required String questionId,
    required String chosenKey,
    String? planItemId,
    int? responseMs,
  }) async {
    final key = _uuid.v4();
    final event = AnswerEvent(
      planItemId: planItemId,
      questionId: questionId,
      chosenKey: chosenKey,
      responseMs: responseMs,
    );
    try {
      return await _api.postAnswer(event, idempotencyKey: key);
    } on NetworkError {
      await _queue.enqueue(
        QueuedEvent(
          id: key,
          kind: QueuedKind.answer,
          body: event.toJson(),
          createdAt: _clock().millisecondsSinceEpoch,
        ),
      );
      return null;
    }
  }

  Future<void> dispose() async {
    await _sub?.cancel();
    _sub = null;
  }
}

final offlineQueueProvider = Provider<OfflineQueue>((ref) {
  final api = ref.watch(apiProvider);
  return OfflineQueue(
    store: ref.watch(keyValueStoreProvider),
    send: (e) => sendQueuedEvent(api, e),
  );
});

final telemetryProvider = Provider<TelemetryService>((ref) {
  final service = TelemetryService(
    api: ref.watch(apiProvider),
    queue: ref.watch(offlineQueueProvider),
    connectivity: ref.watch(connectivityProvider),
  );
  ref.onDispose(service.dispose);
  return service;
});
