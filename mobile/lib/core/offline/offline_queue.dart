import 'dart:async';
import 'dart:convert';

import '../api/models/models.dart';
import '../errors/app_error.dart';
import 'key_value_store.dart';

enum QueuedKind {
  watch('watch'),
  answer('answer');

  const QueuedKind(this.wire);
  final String wire;

  static QueuedKind parse(Object? v) =>
      values.firstWhere((e) => e.wire == v, orElse: () => QueuedKind.watch);
}

/// One learner event waiting to reach the API. `id` is the Idempotency-Key:
/// it is minted once, persisted, and reused on every replay so the server
/// never records the same event twice.
class QueuedEvent {
  const QueuedEvent({
    required this.id,
    required this.kind,
    required this.body,
    required this.createdAt,
  });

  factory QueuedEvent.fromJson(Json j) => QueuedEvent(
        id: j['id'] as String,
        kind: QueuedKind.parse(j['kind']),
        body: Map<String, dynamic>.from(j['body'] as Map<dynamic, dynamic>),
        createdAt: j['created_at'] as int,
      );

  final String id;
  final QueuedKind kind;
  final Json body;
  final int createdAt;

  Json toJson() => {
        'id': id,
        'kind': kind.wire,
        'body': body,
        'created_at': createdAt,
      };
}

class ReplayResult {
  const ReplayResult({
    required this.sent,
    required this.dropped,
    required this.remaining,
  });
  final int sent;
  final int dropped;
  final int remaining;
}

typedef EventSender = Future<void> Function(QueuedEvent event);

/// Persistent FIFO of telemetry events.
///
/// * `enqueue` persists before returning, so a crash never loses an event.
/// * `replay` sends in order. A [NetworkError] stops the pass and keeps the
///   rest; an [ApiError] (the server saw it and rejected it, e.g. 409 already
///   answered) drops the event; success removes it. Only one replay runs at a
///   time, so an event is never in flight twice.
class OfflineQueue {
  OfflineQueue({
    required KeyValueStore store,
    required EventSender send,
    this.storageKey = 'reelearn.telemetry_queue',
  })  : _store = store,
        _send = send;

  final KeyValueStore _store;
  final EventSender _send;
  final String storageKey;

  final List<QueuedEvent> _events = [];
  bool _loaded = false;
  Future<ReplayResult>? _inFlight;

  List<QueuedEvent> get pending => List.unmodifiable(_events);
  bool get isEmpty => _events.isEmpty;

  Future<void> load() async {
    if (_loaded) return;
    _loaded = true;
    final raw = await _store.getString(storageKey);
    if (raw == null || raw.isEmpty) return;
    final decoded = jsonDecode(raw);
    if (decoded is! List) return;
    _events
      ..clear()
      ..addAll(
        decoded.whereType<Map<String, dynamic>>().map(QueuedEvent.fromJson),
      );
  }

  Future<void> enqueue(QueuedEvent event) async {
    await load();
    if (_events.any((e) => e.id == event.id)) return;
    _events.add(event);
    await _persist();
  }

  Future<ReplayResult> replay() {
    final running = _inFlight;
    if (running != null) return running;
    final f = _replay().whenComplete(() => _inFlight = null);
    _inFlight = f;
    return f;
  }

  Future<ReplayResult> _replay() async {
    await load();
    var sent = 0;
    var dropped = 0;
    for (final event in List<QueuedEvent>.of(_events)) {
      try {
        await _send(event);
        _events.removeWhere((e) => e.id == event.id);
        sent++;
        await _persist();
      } on NetworkError {
        break;
      } on ApiError catch (e) {
        if (e.isUnauthorized) break;
        _events.removeWhere((x) => x.id == event.id);
        dropped++;
        await _persist();
      } on SessionLostError {
        break;
      }
    }
    return ReplayResult(
      sent: sent,
      dropped: dropped,
      remaining: _events.length,
    );
  }

  Future<void> clear() async {
    _events.clear();
    await _persist();
  }

  Future<void> _persist() => _store.setString(
        storageKey,
        jsonEncode(_events.map((e) => e.toJson()).toList()),
      );
}
