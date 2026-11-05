import 'dart:async';

import '../errors/app_error.dart';
import 'fixtures.dart';
import 'models/models.dart';
import 'reelearn_api.dart';

/// In-memory, deterministic [ReelearnApi]. Selected with
/// `--dart-define=API=fake`; used by every test.
///
/// Behaviour mirrors the real API: OTP `1234`, five wrong codes lock the
/// challenge (423), three OTP requests per phone per hour (429), one nudge
/// per person per day, 409 on a second answer to the same plan item, and
/// idempotency keys that replay the first response.
class FakeApi implements ReelearnApi {
  FakeApi({DateTime? now, this.latency = Duration.zero})
      : _now = now ?? DateTime(2026, 10, 30, 9);

  static const otpCode = '1234';
  static const maxOtpAttempts = 5;
  static const maxOtpRequestsPerHour = 3;
  static const challengeIdForSara = '0f3a3c2e-0000-4000-8000-00000000ch10';
  static const challengeIdForKhalid = '0f3a3c2e-0000-4000-8000-00000000ch02';

  final DateTime _now;
  final Duration latency;

  /// When true every call fails with [NetworkError]; used by queue tests.
  bool offline = false;

  Me? _user;
  int _tokenGeneration = 0;
  final Map<String, _Challenge> _challenges = {};
  final Map<String, int> _otpRequests = {};

  DailyPlan _plan = Fixtures.plan;
  UserStats _stats = Fixtures.stats;
  final Set<String> _answeredItems = {};
  final Map<String, Json> _idempotent = {};
  final List<WatchEvent> watchEvents = [];
  final List<AnswerEvent> answerEvents = [];
  final List<String> reelRequests = [];
  final Set<String> _saved = {
    for (final c in Fixtures.library)
      if (c.saved) c.id,
  };
  TeamReadiness _team = Fixtures.team;
  final List<String> nudgedUserIds = [];
  final List<MePatch> patches = [];
  int _recordingSeq = 0;

  // ------------------------------------------------------------ test seams

  Me? get currentUser => _user;
  DailyPlan get planSnapshot => _plan;
  Set<String> get savedIds => Set.unmodifiable(_saved);

  /// Sign in without the OTP dance (for tests and `API=fake` dev builds).
  TokenPair signInAs(Me user) {
    _user = user;
    return _issue(user);
  }

  // ------------------------------------------------------------ helpers

  Future<T> _run<T>(FutureOr<T> Function() body) async {
    if (latency > Duration.zero) await Future<void>.delayed(latency);
    if (offline) throw const NetworkError('fake offline');
    return body();
  }

  Never _problem(int status, String title, {String? detail}) => throw ApiError(
        status: status,
        requestId: 'fake-req-${_idempotent.length}',
        problem: Problem(
          type:
              'https://reelearn.sa/problems/${title.toLowerCase().replaceAll(' ', '-')}',
          title: title,
          status: status,
          detail: detail,
        ),
      );

  Me _requireUser() {
    final u = _user;
    if (u == null) _problem(401, 'Unauthorized', detail: 'Sign in first.');
    return u;
  }

  Me _requireManager() {
    final u = _requireUser();
    if (!u.isManager) _problem(403, 'Forbidden', detail: 'Managers only.');
    return u;
  }

  TokenPair _issue(Me user) {
    _tokenGeneration++;
    return TokenPair(
      accessToken: 'fake-access-${user.id}-$_tokenGeneration',
      refreshToken: 'fake-refresh-${user.id}-$_tokenGeneration',
      expiresIn: 900,
      user: user,
    );
  }

  static String normalisePhone(String raw) {
    var digits = raw.replaceAll(RegExp(r'[^0-9+]'), '');
    if (digits.startsWith('00')) digits = '+${digits.substring(2)}';
    if (digits.startsWith('05') && digits.length == 10) {
      digits = '+966${digits.substring(1)}';
    } else if (digits.startsWith('5') && digits.length == 9) {
      digits = '+966$digits';
    } else if (digits.startsWith('966')) {
      digits = '+$digits';
    }
    return digits;
  }

  String _date(DateTime d) =>
      '${d.year}-${d.month.toString().padLeft(2, '0')}-${d.day.toString().padLeft(2, '0')}';

  // ------------------------------------------------------------ auth

  @override
  Future<TenantPublic> lookupTenant({String? slug, String? inviteToken}) =>
      _run(() {
        final s = slug?.trim().toLowerCase();
        if (s == Fixtures.tenant.slug ||
            (inviteToken?.startsWith('inv') ?? false)) {
          return Fixtures.tenant;
        }
        _problem(
          404,
          'Company not found',
          detail: 'No company space with that code.',
        );
      });

  @override
  Future<OtpChallenge> requestOtp({
    required String tenantSlug,
    String? phone,
  }) =>
      _run(() {
        if (tenantSlug != Fixtures.tenant.slug) {
          _problem(404, 'Company not found');
        }
        final p = normalisePhone(phone ?? '');
        final Me? user = switch (p) {
          Fixtures.saraPhone => Fixtures.sara,
          Fixtures.khalidPhone => Fixtures.khalid,
          _ => null,
        };
        if (user == null) {
          _problem(
            404,
            'Number not found',
            detail: "This number isn't on your company's list.",
          );
        }
        final count = (_otpRequests[p] ?? 0) + 1;
        _otpRequests[p] = count;
        if (count > maxOtpRequestsPerHour) {
          _problem(429, 'Too many requests', detail: 'Three codes per hour.');
        }
        final id = user.id == Fixtures.saraId
            ? challengeIdForSara
            : challengeIdForKhalid;
        _challenges[id] = _Challenge(user: user);
        return OtpChallenge(
          challengeId: id,
          channel: OtpChannel.whatsapp,
          expiresInSeconds: 300,
        );
      });

  @override
  Future<TokenPair> verifyOtp({
    required String challengeId,
    required String code,
    required DeviceInfo device,
  }) =>
      _run(() {
        final c = _challenges[challengeId];
        if (c == null) {
          _problem(400, 'Code expired', detail: 'Ask for a new code.');
        }
        if (c.locked) {
          _problem(
            423,
            'Locked',
            detail: 'Too many tries. Try again in 15 minutes.',
          );
        }
        if (code.trim() != otpCode) {
          c.attempts++;
          if (c.attempts >= maxOtpAttempts) {
            c.locked = true;
            _problem(
              423,
              'Locked',
              detail: 'Too many tries. Try again in 15 minutes.',
            );
          }
          _problem(
            400,
            'Wrong code',
            detail: '${maxOtpAttempts - c.attempts} attempts left.',
          );
        }
        _challenges.remove(challengeId);
        _user = c.user;
        return _issue(c.user);
      });

  @override
  Future<TokenPair> refresh(String refreshToken) => _run(() {
        final u = _user;
        if (u == null || !refreshToken.startsWith('fake-refresh-${u.id}-')) {
          _problem(401, 'Unauthorized', detail: 'Refresh token rejected.');
        }
        return _issue(u);
      });

  @override
  Future<void> logout({bool allDevices = false}) => _run(() {
        _user = null;
      });

  // ------------------------------------------------------------ me

  @override
  Future<Me> getMe() => _run(_requireUser);

  @override
  Future<Me> patchMe(MePatch patch) => _run(() {
        final u = _requireUser();
        patches.add(patch);
        return _user = u.apply(patch);
      });

  @override
  Future<PrivacyStatement> getPrivacy() => _run(() {
        _requireUser();
        return Fixtures.privacy;
      });

  @override
  Future<Export> requestExport() => _run(() {
        _requireUser();
        return Export(
          id: '0f3a3c2e-0000-4000-8000-0000000000ex',
          kind: 'personal',
          status: 'queued',
          createdAt: _now.toIso8601String(),
        );
      });

  // ------------------------------------------------------------ learn

  @override
  Future<DailyPlan> getToday() => _run(() {
        _requireUser();
        return _plan;
      });

  @override
  Future<void> postWatch(WatchEvent event, {required String idempotencyKey}) =>
      _run(() {
        _requireUser();
        if (_idempotent.containsKey(idempotencyKey)) return;
        _idempotent[idempotencyKey] = const {};
        watchEvents.add(event);
        final itemId = event.planItemId;
        if (itemId != null) {
          _plan = _plan.copyWith(
            items: [
              for (final i in _plan.items)
                if (i.id == itemId && i.state == PlanItemState.pending)
                  i.copyWith(state: PlanItemState.watched)
                else
                  i,
            ],
          );
        }
      });

  @override
  Future<AnswerOutcome> postAnswer(
    AnswerEvent event, {
    required String idempotencyKey,
  }) =>
      _run(() {
        _requireUser();
        final replay = _idempotent[idempotencyKey];
        if (replay != null) return AnswerOutcome.fromJson(replay);

        final itemId = event.planItemId;
        if (itemId != null && _answeredItems.contains(itemId)) {
          _problem(
            409,
            'Already answered',
            detail: 'This question was answered today.',
          );
        }
        final correct = Fixtures.correctKeys[event.questionId];
        if (correct == null) _problem(404, 'Question not found');

        answerEvents.add(event);
        final isCorrect = event.chosenKey == correct;
        final item = _plan.items.where((i) => i.id == itemId).firstOrNull;
        final isReview = item?.reason == PlanItemReason.review;
        final nextDays = isCorrect ? (isReview ? 30 : 3) : 3;

        _stats = _stats.copyWith(
          rememberedCount: _stats.rememberedCount + (isCorrect ? 1 : 0),
          lastActiveOn: _date(_now),
        );
        if (itemId != null) {
          _answeredItems.add(itemId);
          final items = [
            for (final i in _plan.items)
              if (i.id == itemId)
                i.copyWith(state: PlanItemState.answered)
              else
                i,
          ];
          final allDone = items.every(
            (i) =>
                i.effectiveQuestion == null ||
                i.state == PlanItemState.answered,
          );
          _plan = _plan.copyWith(
            items: items,
            stats: _stats,
            status: allDone ? PlanStatus.done : PlanStatus.open,
          );
        }
        final outcome = AnswerOutcome(
          isCorrect: isCorrect,
          correctKey: correct,
          explanation: Fixtures.explanations[event.questionId],
          nextDueOn: _date(_now.add(Duration(days: nextDays))),
          planStatus: _plan.status == PlanStatus.done
              ? PlanStatus.done
              : PlanStatus.open,
          stats: _stats,
        );
        _idempotent[idempotencyKey] = outcome.toJson();
        return outcome;
      });

  @override
  Future<Progress> getProgress() => _run(() {
        _requireUser();
        return Progress(
          rememberedPct: Fixtures.progress.rememberedPct,
          series: Fixtures.progress.series,
          byTopic: Fixtures.progress.byTopic,
          milestones: Fixtures.progress.milestones,
          stats: _stats,
        );
      });

  @override
  Future<Page<ReelCard>> getLibrary({
    String? topicId,
    String? query,
    bool? saved,
    String? cursor,
    int? limit,
  }) =>
      _run(() {
        _requireUser();
        final q = query?.trim().toLowerCase() ?? '';
        final items = Fixtures.library
            .map((c) => c.copyWith(saved: _saved.contains(c.id)))
            .where(
              (c) =>
                  topicId == null ||
                  Fixtures.topicIdsByName[c.topicName] == topicId,
            )
            .where((c) => saved == null || c.saved == saved)
            .where(
              (c) =>
                  q.isEmpty ||
                  c.title.toLowerCase().contains(q) ||
                  c.topicName.toLowerCase().contains(q),
            )
            .toList(growable: false);
        return Page(items: items);
      });

  @override
  Future<void> saveReel(String reelId) => _run(() {
        _requireUser();
        _saved.add(reelId);
      });

  @override
  Future<void> unsaveReel(String reelId) => _run(() {
        _requireUser();
        _saved.remove(reelId);
      });

  @override
  Future<void> requestReel(String text) => _run(() {
        _requireUser();
        if (text.trim().length < 3) {
          _problem(400, 'Too short', detail: 'Say a little more.');
        }
        reelRequests.add(text.trim());
      });

  @override
  Future<ReelPlayable> getReel(String reelId, {String? lang}) => _run(() {
        _requireUser();
        final reel = switch (reelId) {
          Fixtures.reel1Id => Fixtures.reel1,
          Fixtures.reel2Id => Fixtures.reel2,
          _ => null,
        };
        if (reel == null) _problem(404, 'Reel not found');
        return reel.copyWith(saved: _saved.contains(reelId));
      });

  // ------------------------------------------------------------ manager

  @override
  Future<TeamReadiness> getTeam() => _run(() {
        _requireManager();
        return _team;
      });

  @override
  Future<NudgeResult> nudge(List<String> userIds) => _run(() {
        _requireManager();
        var queued = 0;
        var skipped = 0;
        final members = _team.members.map((m) {
          if (!userIds.contains(m.userId)) return m;
          if (m.nudgedToday) {
            skipped++;
            return m;
          }
          queued++;
          nudgedUserIds.add(m.userId);
          return m.copyWith(nudgedToday: true);
        }).toList(growable: false);
        _team = _team.copyWith(members: members);
        return NudgeResult(queued: queued, skippedAlreadyNudged: skipped);
      });

  @override
  Future<List<ReelRequest>> getTeamReelRequests() => _run(() {
        _requireManager();
        return [
          for (final (i, t) in reelRequests.indexed)
            ReelRequest(id: 'req-$i', text: t, votes: 1),
        ];
      });

  // ------------------------------------------------------------ sme

  @override
  Future<UploadSlot> createRecording(RecordingRequest request) => _run(() {
        final u = _requireUser();
        if (!u.canRecord) _problem(403, 'Forbidden');
        if (!request.consentConfirmed) {
          _problem(
            400,
            'Consent required',
            detail: 'Tick the consent box first.',
          );
        }
        _recordingSeq++;
        return UploadSlot(
          id: '0f3a3c2e-0000-4000-8000-0000000rec0$_recordingSeq',
          uploadUrl: 'https://uploads.fake.reelearn.sa/rec$_recordingSeq',
          method: 'PUT',
          headers: const {'Content-Type': 'video/mp4'},
          expiresAt: _now.add(const Duration(minutes: 30)).toIso8601String(),
        );
      });

  @override
  Future<void> uploadToSlot(UploadSlot slot, List<int> bytes) => _run(() {});

  @override
  Future<DraftReel> submitRecording(String recordingId) => _run(() {
        _requireUser();
        return DraftReel(
          id: recordingId,
          title: 'Phone recording',
          status: 'draft',
        );
      });
}

class _Challenge {
  _Challenge({required this.user});
  final Me user;
  int attempts = 0;
  bool locked = false;
}
