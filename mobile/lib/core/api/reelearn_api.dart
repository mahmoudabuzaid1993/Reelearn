import 'models/models.dart';

/// Body of `POST /me/watch`.
class WatchEvent {
  const WatchEvent({
    required this.reelId,
    required this.watchedMs,
    required this.completed,
    this.planItemId,
    this.channel = Channel.app,
  });

  factory WatchEvent.fromJson(Json j) => WatchEvent(
        planItemId: j['plan_item_id'] as String?,
        reelId: j['reel_id'] as String,
        watchedMs: j['watched_ms'] as int,
        completed: j['completed'] as bool,
        channel: Channel.parse(j['channel']),
      );

  final String? planItemId;
  final String reelId;
  final int watchedMs;
  final bool completed;
  final Channel channel;

  Json toJson() => {
        if (planItemId != null) 'plan_item_id': planItemId,
        'reel_id': reelId,
        'watched_ms': watchedMs,
        'completed': completed,
        'channel': channel.wire,
      };
}

/// Body of `POST /me/answers`.
class AnswerEvent {
  const AnswerEvent({
    required this.questionId,
    required this.chosenKey,
    this.planItemId,
    this.responseMs,
    this.channel = Channel.app,
  });

  factory AnswerEvent.fromJson(Json j) => AnswerEvent(
        planItemId: j['plan_item_id'] as String?,
        questionId: j['question_id'] as String,
        chosenKey: j['chosen_key'] as String,
        responseMs: j['response_ms'] as int?,
        channel: Channel.parse(j['channel']),
      );

  final String? planItemId;
  final String questionId;
  final String chosenKey;
  final int? responseMs;
  final Channel channel;

  Json toJson() => {
        if (planItemId != null) 'plan_item_id': planItemId,
        'question_id': questionId,
        'chosen_key': chosenKey,
        if (responseMs != null) 'response_ms': responseMs,
        'channel': channel.wire,
      };
}

class RecordingRequest {
  const RecordingRequest({
    required this.topicId,
    required this.mime,
    required this.bytes,
    required this.consentConfirmed,
    this.title,
  });

  final String topicId;
  final String? title;
  final String mime;
  final int bytes;
  final bool consentConfirmed;

  Json toJson() => {
        'topic_id': topicId,
        if (title != null) 'title': title,
        'mime': mime,
        'bytes': bytes,
        'consent_confirmed': consentConfirmed,
      };
}

/// The only door to the network. Implemented by [HttpApi] and [FakeApi].
abstract class ReelearnApi {
  // auth
  Future<TenantPublic> lookupTenant({String? slug, String? inviteToken});
  Future<OtpChallenge> requestOtp({required String tenantSlug, String? phone});
  Future<TokenPair> verifyOtp({
    required String challengeId,
    required String code,
    required DeviceInfo device,
  });
  Future<TokenPair> refresh(String refreshToken);
  Future<void> logout({bool allDevices = false});

  // me
  Future<Me> getMe();
  Future<Me> patchMe(MePatch patch);
  Future<PrivacyStatement> getPrivacy();
  Future<Export> requestExport();

  // learn
  Future<DailyPlan> getToday();
  Future<void> postWatch(WatchEvent event, {required String idempotencyKey});
  Future<AnswerOutcome> postAnswer(
    AnswerEvent event, {
    required String idempotencyKey,
  });
  Future<Progress> getProgress();
  Future<Page<ReelCard>> getLibrary({
    String? topicId,
    String? query,
    bool? saved,
    String? cursor,
    int? limit,
  });
  Future<void> saveReel(String reelId);
  Future<void> unsaveReel(String reelId);
  Future<void> requestReel(String text);
  Future<ReelPlayable> getReel(String reelId, {String? lang});

  // manager
  Future<TeamReadiness> getTeam();
  Future<NudgeResult> nudge(List<String> userIds);
  Future<List<ReelRequest>> getTeamReelRequests();

  // sme
  Future<UploadSlot> createRecording(RecordingRequest request);
  Future<void> uploadToSlot(UploadSlot slot, List<int> bytes);
  Future<DraftReel> submitRecording(String recordingId);
}
