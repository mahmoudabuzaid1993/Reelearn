import 'package:dio/dio.dart';

import 'dio_client.dart';
import 'models/models.dart';
import 'reelearn_api.dart';

/// [ReelearnApi] over HTTP. Every method is one endpoint of the OpenAPI
/// contract; nothing here knows about widgets or providers.
class HttpApi implements ReelearnApi {
  HttpApi(this._dio, {Dio? uploadDio}) : _upload = uploadDio ?? Dio();

  final Dio _dio;

  /// Pre-signed PUTs go to object storage, not the API: no auth, no base URL.
  final Dio _upload;

  Future<T> _call<T>(Future<T> Function() run) async {
    try {
      return await run();
    } on DioException catch (e) {
      throw mapDioError(e);
    }
  }

  Json _json(Response<dynamic> res) {
    final d = res.data;
    return d is Map<String, dynamic> ? d : const {};
  }

  // ---------------------------------------------------------------- auth

  @override
  Future<TenantPublic> lookupTenant({String? slug, String? inviteToken}) =>
      _call(() async {
        final res = await _dio.post<dynamic>(
          '/auth/tenant/lookup',
          data: {
            if (slug != null) 'slug': slug,
            if (inviteToken != null) 'invite_token': inviteToken,
          },
        );
        return TenantPublic.fromJson(_json(res));
      });

  @override
  Future<OtpChallenge> requestOtp({
    required String tenantSlug,
    String? phone,
  }) =>
      _call(() async {
        final res = await _dio.post<dynamic>(
          '/auth/otp/request',
          data: {'tenant_slug': tenantSlug, if (phone != null) 'phone': phone},
        );
        return OtpChallenge.fromJson(_json(res));
      });

  @override
  Future<TokenPair> verifyOtp({
    required String challengeId,
    required String code,
    required DeviceInfo device,
  }) =>
      _call(() async {
        final res = await _dio.post<dynamic>(
          '/auth/otp/verify',
          data: {
            'challenge_id': challengeId,
            'code': code,
            'device': device.toJson(),
          },
        );
        return TokenPair.fromJson(_json(res));
      });

  @override
  Future<TokenPair> refresh(String refreshToken) => _call(() async {
        final res = await _dio.post<dynamic>(
          '/auth/refresh',
          data: {'refresh_token': refreshToken},
        );
        return TokenPair.fromJson(_json(res));
      });

  @override
  Future<void> logout({bool allDevices = false}) => _call(
        () => _dio
            .post<dynamic>('/auth/logout', data: {'all_devices': allDevices}),
      );

  // ---------------------------------------------------------------- me

  @override
  Future<Me> getMe() =>
      _call(() async => Me.fromJson(_json(await _dio.get<dynamic>('/me'))));

  @override
  Future<Me> patchMe(MePatch patch) => _call(() async {
        final res = await _dio.patch<dynamic>('/me', data: patch.toJson());
        return Me.fromJson(_json(res));
      });

  @override
  Future<PrivacyStatement> getPrivacy() => _call(
        () async => PrivacyStatement.fromJson(
          _json(await _dio.get<dynamic>('/me/privacy')),
        ),
      );

  @override
  Future<Export> requestExport() => _call(
        () async =>
            Export.fromJson(_json(await _dio.post<dynamic>('/me/export'))),
      );

  // ---------------------------------------------------------------- learn

  @override
  Future<DailyPlan> getToday() => _call(
        () async =>
            DailyPlan.fromJson(_json(await _dio.get<dynamic>('/me/today'))),
      );

  @override
  Future<void> postWatch(WatchEvent event, {required String idempotencyKey}) =>
      _call(
        () => _dio.post<dynamic>(
          '/me/watch',
          data: event.toJson(),
          options: Options(headers: {'Idempotency-Key': idempotencyKey}),
        ),
      );

  @override
  Future<AnswerOutcome> postAnswer(
    AnswerEvent event, {
    required String idempotencyKey,
  }) =>
      _call(() async {
        final res = await _dio.post<dynamic>(
          '/me/answers',
          data: event.toJson(),
          options: Options(headers: {'Idempotency-Key': idempotencyKey}),
        );
        return AnswerOutcome.fromJson(_json(res));
      });

  @override
  Future<Progress> getProgress() => _call(
        () async =>
            Progress.fromJson(_json(await _dio.get<dynamic>('/me/progress'))),
      );

  @override
  Future<Page<ReelCard>> getLibrary({
    String? topicId,
    String? query,
    bool? saved,
    String? cursor,
    int? limit,
  }) =>
      _call(() async {
        final res = await _dio.get<dynamic>(
          '/me/library',
          queryParameters: {
            if (topicId != null) 'topic_id': topicId,
            if (query != null && query.isNotEmpty) 'q': query,
            if (saved != null) 'saved': saved,
            if (cursor != null) 'cursor': cursor,
            if (limit != null) 'limit': limit,
          },
        );
        return Page.fromJson(_json(res), ReelCard.fromJson);
      });

  @override
  Future<void> saveReel(String reelId) =>
      _call(() => _dio.put<dynamic>('/me/saved/$reelId'));

  @override
  Future<void> unsaveReel(String reelId) =>
      _call(() => _dio.delete<dynamic>('/me/saved/$reelId'));

  @override
  Future<void> requestReel(String text) => _call(
        () => _dio.post<dynamic>('/me/reel-requests', data: {'text': text}),
      );

  @override
  Future<ReelPlayable> getReel(String reelId, {String? lang}) =>
      _call(() async {
        final res = await _dio.get<dynamic>(
          '/reels/$reelId',
          queryParameters: {if (lang != null) 'lang': lang},
        );
        return ReelPlayable.fromJson(_json(res));
      });

  // ---------------------------------------------------------------- manager

  @override
  Future<TeamReadiness> getTeam() => _call(
        () async => TeamReadiness.fromJson(
          _json(await _dio.get<dynamic>('/manager/team')),
        ),
      );

  @override
  Future<NudgeResult> nudge(List<String> userIds) => _call(() async {
        final res = await _dio.post<dynamic>(
          '/manager/nudges',
          data: {'user_ids': userIds},
        );
        return NudgeResult.fromJson(_json(res));
      });

  @override
  Future<List<ReelRequest>> getTeamReelRequests() => _call(() async {
        final res = await _dio.get<dynamic>('/manager/reel-requests');
        final data = res.data;
        if (data is! List) return const [];
        return data
            .whereType<Map<String, dynamic>>()
            .map(ReelRequest.fromJson)
            .toList(growable: false);
      });

  // ---------------------------------------------------------------- sme

  @override
  Future<UploadSlot> createRecording(RecordingRequest request) =>
      _call(() async {
        final res =
            await _dio.post<dynamic>('/sme/recordings', data: request.toJson());
        return UploadSlot.fromJson(_json(res));
      });

  @override
  Future<void> uploadToSlot(UploadSlot slot, List<int> bytes) => _call(
        () => _upload.put<dynamic>(
          slot.uploadUrl,
          data: Stream.fromIterable([bytes]),
          options: Options(
            headers: {
              ...slot.headers,
              Headers.contentLengthHeader: bytes.length,
            },
          ),
        ),
      );

  @override
  Future<DraftReel> submitRecording(String recordingId) => _call(() async {
        final res =
            await _dio.post<dynamic>('/sme/recordings/$recordingId/submit');
        return DraftReel.fromJson(_json(res));
      });
}
