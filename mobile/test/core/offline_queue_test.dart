import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:reelearn_mobile/core/api/fake_api.dart';
import 'package:reelearn_mobile/core/api/fixtures.dart';
import 'package:reelearn_mobile/core/api/models/models.dart';
import 'package:reelearn_mobile/core/api/reelearn_api.dart';
import 'package:reelearn_mobile/core/errors/app_error.dart';
import 'package:reelearn_mobile/core/offline/connectivity.dart';
import 'package:reelearn_mobile/core/offline/key_value_store.dart';
import 'package:reelearn_mobile/core/offline/offline_queue.dart';
import 'package:reelearn_mobile/features/player/data/telemetry.dart';

QueuedEvent event(String id) => QueuedEvent(
      id: id,
      kind: QueuedKind.watch,
      body: {'reel_id': Fixtures.reel1Id, 'watched_ms': 0, 'completed': false},
      createdAt: 1,
    );

void main() {
  group('OfflineQueue', () {
    test('persists events and reloads them from the store', () async {
      final store = MemoryKeyValueStore();
      final q = OfflineQueue(store: store, send: (_) async {});
      await q.enqueue(event('a'));
      await q.enqueue(event('b'));
      await q.enqueue(event('a')); // duplicate id ignored
      expect(q.pending.map((e) => e.id), ['a', 'b']);

      final raw = await store.getString(q.storageKey);
      expect((jsonDecode(raw!) as List).length, 2);

      final reloaded = OfflineQueue(store: store, send: (_) async {});
      await reloaded.load();
      expect(reloaded.pending.map((e) => e.id), ['a', 'b']);
    });

    test('replays each event exactly once with a stable idempotency key',
        () async {
      final store = MemoryKeyValueStore();
      final sent = <String>[];
      final q = OfflineQueue(store: store, send: (e) async => sent.add(e.id));
      await q.enqueue(event('a'));
      await q.enqueue(event('b'));

      final first = await q.replay();
      expect(first.sent, 2);
      expect(first.remaining, 0);
      final second = await q.replay();
      expect(second.sent, 0);
      expect(sent, ['a', 'b']);
      expect(await store.getString(q.storageKey), '[]');
    });

    test('a network error stops the pass; the next replay reuses the same ids',
        () async {
      final store = MemoryKeyValueStore();
      final sent = <String>[];
      var offline = true;
      final q = OfflineQueue(
        store: store,
        send: (e) async {
          if (offline) throw const NetworkError();
          sent.add(e.id);
        },
      );
      await q.enqueue(event('a'));
      await q.enqueue(event('b'));
      final r1 = await q.replay();
      expect(r1.sent, 0);
      expect(r1.remaining, 2);

      offline = false;
      final r2 = await q.replay();
      expect(r2.sent, 2);
      expect(sent, ['a', 'b']);
    });

    test('a 4xx the server rejected is dropped, not retried forever', () async {
      final q = OfflineQueue(
        store: MemoryKeyValueStore(),
        send: (e) async {
          if (e.id == 'dup') throw const ApiError(status: 409);
        },
      );
      await q.enqueue(event('dup'));
      await q.enqueue(event('ok'));
      final r = await q.replay();
      expect(r.dropped, 1);
      expect(r.sent, 1);
      expect(q.isEmpty, isTrue);
    });

    test('concurrent replays never send an event twice', () async {
      final sent = <String>[];
      final q = OfflineQueue(
        store: MemoryKeyValueStore(),
        send: (e) async {
          await Future<void>.delayed(const Duration(milliseconds: 5));
          sent.add(e.id);
        },
      );
      await q.enqueue(event('a'));
      await q.enqueue(event('b'));
      await Future.wait([q.replay(), q.replay(), q.replay()]);
      expect(sent, ['a', 'b']);
    });
  });

  group('TelemetryService', () {
    test('watch events queue offline and replay once on reconnect', () async {
      final api = FakeApi()..signInAs(Fixtures.sara);
      final connectivity = FakeConnectivityWatcher(initiallyOnline: false);
      final queue = OfflineQueue(
        store: MemoryKeyValueStore(),
        send: (e) => sendQueuedEvent(api, e),
      );
      final t =
          TelemetryService(api: api, queue: queue, connectivity: connectivity);
      await t.start();

      api.offline = true;
      await t.recordWatch(
        reelId: Fixtures.reel1Id,
        watchedMs: 0,
        completed: false,
      );
      await t.recordWatch(
        reelId: Fixtures.reel1Id,
        watchedMs: 41000,
        completed: true,
      );
      await Future<void>.delayed(Duration.zero);
      expect(queue.pending.length, 2);
      expect(api.watchEvents, isEmpty);

      api.offline = false;
      connectivity.set(online: true);
      await Future<void>.delayed(const Duration(milliseconds: 10));
      expect(api.watchEvents.length, 2);
      expect(api.watchEvents.last.completed, isTrue);
      expect(queue.isEmpty, isTrue);

      // Replaying again (same keys) is a no-op server-side.
      await queue.replay();
      expect(api.watchEvents.length, 2);
      await t.dispose();
      await connectivity.dispose();
    });

    test('an answer picked offline is queued and reveals nothing', () async {
      final api = FakeApi()..signInAs(Fixtures.sara);
      final connectivity = FakeConnectivityWatcher();
      final queue = OfflineQueue(
        store: MemoryKeyValueStore(),
        send: (e) => sendQueuedEvent(api, e),
      );
      final t =
          TelemetryService(api: api, queue: queue, connectivity: connectivity);
      api.offline = true;
      final outcome = await t.answer(
        planItemId: Fixtures.item1Id,
        questionId: Fixtures.question1Id,
        chosenKey: 'B',
      );
      expect(outcome, isNull);
      expect(queue.pending.single.kind, QueuedKind.answer);

      api.offline = false;
      await queue.replay();
      expect(api.answerEvents.single.chosenKey, 'B');
      expect(api.planSnapshot.items.first.state, PlanItemState.answered);
      await t.dispose();
      await connectivity.dispose();
    });

    test('online answers return the outcome without touching the queue',
        () async {
      final api = FakeApi()..signInAs(Fixtures.sara);
      final queue = OfflineQueue(
        store: MemoryKeyValueStore(),
        send: (e) => sendQueuedEvent(api, e),
      );
      final t = TelemetryService(
        api: api,
        queue: queue,
        connectivity: FakeConnectivityWatcher(),
      );
      final outcome = await t.answer(
        planItemId: Fixtures.item1Id,
        questionId: Fixtures.question1Id,
        chosenKey: 'A',
      );
      expect(outcome!.isCorrect, isFalse);
      expect(outcome.correctKey, 'B');
      expect(queue.isEmpty, isTrue);
      // The API refuses a second answer to the same plan item.
      await expectLater(
        () => api.postAnswer(
          const AnswerEvent(
            planItemId: Fixtures.item1Id,
            questionId: Fixtures.question1Id,
            chosenKey: 'B',
          ),
          idempotencyKey: 'other-key',
        ),
        throwsA(isA<ApiError>().having((e) => e.status, 'status', 409)),
      );
    });
  });
}
