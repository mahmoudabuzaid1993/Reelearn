import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:uuid/uuid.dart';

import '../../../core/api/models/models.dart';
import '../../../core/api/reelearn_api.dart';
import '../../../core/offline/key_value_store.dart';
import '../../../core/providers.dart';

/// Auth calls plus the per-install device id the refresh token is bound to.
class AuthRepository {
  AuthRepository({required ReelearnApi api, required KeyValueStore kv})
      : _api = api,
        _kv = kv;

  static const _deviceKey = 'reelearn.device_id';

  final ReelearnApi _api;
  final KeyValueStore _kv;

  Future<DeviceInfo> deviceInfo() async {
    var id = await _kv.getString(_deviceKey);
    if (id == null) {
      id = const Uuid().v4();
      await _kv.setString(_deviceKey, id);
    }
    final platform = switch (defaultTargetPlatform) {
      TargetPlatform.iOS => 'ios',
      TargetPlatform.android => 'android',
      _ => 'web',
    };
    return DeviceInfo(deviceId: id, platform: platform, appVersion: '0.1.0');
  }

  Future<TenantPublic> lookup({String? slug, String? inviteToken}) =>
      _api.lookupTenant(slug: slug, inviteToken: inviteToken);

  Future<OtpChallenge> requestOtp(String tenantSlug, String phoneE164) =>
      _api.requestOtp(tenantSlug: tenantSlug, phone: phoneE164);

  Future<TokenPair> verify(String challengeId, String code) async =>
      _api.verifyOtp(
        challengeId: challengeId,
        code: code,
        device: await deviceInfo(),
      );
}

final authRepositoryProvider = Provider<AuthRepository>(
  (ref) => AuthRepository(
    api: ref.watch(apiProvider),
    kv: ref.watch(keyValueStoreProvider),
  ),
);
