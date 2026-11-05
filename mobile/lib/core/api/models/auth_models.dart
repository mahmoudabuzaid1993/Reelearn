import 'enums.dart';
import 'json.dart';

/// RFC 9457 problem document.
class Problem {
  const Problem({
    required this.type,
    required this.title,
    required this.status,
    this.detail,
    this.instance,
    this.requestId,
    this.errors = const [],
  });

  factory Problem.fromJson(Json j) => Problem(
        type: asString(j, 'type', fallback: 'about:blank'),
        title: asString(j, 'title'),
        status: asInt(j, 'status'),
        detail: asStringOrNull(j, 'detail'),
        instance: asStringOrNull(j, 'instance'),
        requestId: asStringOrNull(j, 'request_id'),
        errors: asJsonList(j, 'errors')
            .map(
              (e) => FieldError(
                field: asString(e, 'field'),
                message: asString(e, 'message'),
              ),
            )
            .toList(growable: false),
      );

  final String type;
  final String title;
  final int status;
  final String? detail;
  final String? instance;
  final String? requestId;
  final List<FieldError> errors;

  Json toJson() => {
        'type': type,
        'title': title,
        'status': status,
        if (detail != null) 'detail': detail,
        if (instance != null) 'instance': instance,
        if (requestId != null) 'request_id': requestId,
        'errors': errors
            .map((e) => {'field': e.field, 'message': e.message})
            .toList(),
      };
}

class FieldError {
  const FieldError({required this.field, required this.message});
  final String field;
  final String message;
}

class TenantPublic {
  const TenantPublic({
    required this.id,
    required this.slug,
    required this.name,
    required this.languages,
    this.logoUrl,
    this.region = 'ksa',
  });

  factory TenantPublic.fromJson(Json j) => TenantPublic(
        id: asString(j, 'id'),
        slug: asString(j, 'slug'),
        name: asString(j, 'name'),
        logoUrl: asStringOrNull(j, 'logo_url'),
        languages: asStringList(j, 'languages'),
        region: asString(j, 'region', fallback: 'ksa'),
      );

  final String id;
  final String slug;
  final String name;
  final String? logoUrl;
  final List<String> languages;
  final String region;

  Json toJson() => {
        'id': id,
        'slug': slug,
        'name': name,
        'logo_url': logoUrl,
        'languages': languages,
        'region': region,
      };
}

class DeviceInfo {
  const DeviceInfo({
    required this.deviceId,
    required this.platform,
    this.appVersion,
    this.pushToken,
  });

  final String deviceId;
  final String platform;
  final String? appVersion;
  final String? pushToken;

  Json toJson() => {
        'device_id': deviceId,
        'platform': platform,
        if (appVersion != null) 'app_version': appVersion,
        if (pushToken != null) 'push_token': pushToken,
      };
}

class OtpChallenge {
  const OtpChallenge({
    required this.challengeId,
    required this.channel,
    required this.expiresInSeconds,
  });

  factory OtpChallenge.fromJson(Json j) => OtpChallenge(
        challengeId: asString(j, 'challenge_id'),
        channel: OtpChannel.parse(j['channel']),
        expiresInSeconds: asInt(j, 'expires_in_seconds', fallback: 300),
      );

  final String challengeId;
  final OtpChannel channel;
  final int expiresInSeconds;

  Json toJson() => {
        'challenge_id': challengeId,
        'channel': channel.wire,
        'expires_in_seconds': expiresInSeconds,
      };
}

class Me {
  const Me({
    required this.id,
    required this.tenant,
    required this.role,
    required this.fullName,
    required this.preferredLanguage,
    required this.uiLanguage,
    required this.deliveryChannel,
    required this.deliverySlot,
    this.firstName,
    this.phoneE164,
    this.email,
    this.captionsOn = true,
    this.remindersOn = true,
    this.extraReelsOn = false,
    this.pausedUntil,
    this.isFirstOpen = false,
  });

  factory Me.fromJson(Json j) => Me(
        id: asString(j, 'id'),
        tenant: TenantPublic.fromJson(asJsonOrNull(j, 'tenant') ?? const {}),
        role: Role.parse(j['role']),
        fullName: asString(j, 'full_name'),
        firstName: asStringOrNull(j, 'first_name'),
        phoneE164: asStringOrNull(j, 'phone_e164'),
        email: asStringOrNull(j, 'email'),
        preferredLanguage: asString(j, 'preferred_language', fallback: 'en'),
        uiLanguage: asString(j, 'ui_language', fallback: 'en'),
        deliveryChannel: Channel.parse(j['delivery_channel']),
        deliverySlot: DeliverySlot.parse(j['delivery_slot']),
        captionsOn: asBool(j, 'captions_on', fallback: true),
        remindersOn: asBool(j, 'reminders_on', fallback: true),
        extraReelsOn: asBool(j, 'extra_reels_on'),
        pausedUntil: asStringOrNull(j, 'paused_until'),
        isFirstOpen: asBool(j, 'is_first_open'),
      );

  final String id;
  final TenantPublic tenant;
  final Role role;
  final String fullName;
  final String? firstName;
  final String? phoneE164;
  final String? email;
  final String preferredLanguage;
  final String uiLanguage;
  final Channel deliveryChannel;
  final DeliverySlot deliverySlot;
  final bool captionsOn;
  final bool remindersOn;
  final bool extraReelsOn;

  /// ISO date (yyyy-mm-dd) or null.
  final String? pausedUntil;
  final bool isFirstOpen;

  bool get isManager => role == Role.manager || role == Role.admin;
  bool get canRecord => isManager || role == Role.sme;
  String get displayFirstName =>
      firstName ?? (fullName.split(' ').firstOrNull ?? fullName);

  Me apply(MePatch p) => Me(
        id: id,
        tenant: tenant,
        role: role,
        fullName: fullName,
        firstName: firstName,
        phoneE164: phoneE164,
        email: email,
        preferredLanguage: p.preferredLanguage ?? preferredLanguage,
        uiLanguage: p.uiLanguage ?? uiLanguage,
        deliveryChannel: p.deliveryChannel ?? deliveryChannel,
        deliverySlot: p.deliverySlot ?? deliverySlot,
        captionsOn: p.captionsOn ?? captionsOn,
        remindersOn: p.remindersOn ?? remindersOn,
        extraReelsOn: p.extraReelsOn ?? extraReelsOn,
        pausedUntil: p.clearPausedUntil ? null : (p.pausedUntil ?? pausedUntil),
        isFirstOpen: p.markOpened ? false : isFirstOpen,
      );

  Json toJson() => {
        'id': id,
        'tenant': tenant.toJson(),
        'role': role.wire,
        'full_name': fullName,
        'first_name': firstName,
        'phone_e164': phoneE164,
        'email': email,
        'preferred_language': preferredLanguage,
        'ui_language': uiLanguage,
        'delivery_channel': deliveryChannel.wire,
        'delivery_slot': deliverySlot.wire,
        'captions_on': captionsOn,
        'reminders_on': remindersOn,
        'extra_reels_on': extraReelsOn,
        'paused_until': pausedUntil,
        'is_first_open': isFirstOpen,
      };
}

/// PATCH /me body. Only set fields are serialised.
class MePatch {
  const MePatch({
    this.preferredLanguage,
    this.uiLanguage,
    this.deliveryChannel,
    this.deliverySlot,
    this.captionsOn,
    this.remindersOn,
    this.extraReelsOn,
    this.pausedUntil,
    this.clearPausedUntil = false,
    this.pushToken,
    this.markOpened = false,
  });

  final String? preferredLanguage;
  final String? uiLanguage;
  final Channel? deliveryChannel;
  final DeliverySlot? deliverySlot;
  final bool? captionsOn;
  final bool? remindersOn;
  final bool? extraReelsOn;
  final String? pausedUntil;
  final bool clearPausedUntil;
  final String? pushToken;

  /// Client-side flag: the welcome screen was completed. Not sent on the wire.
  final bool markOpened;

  bool get isEmpty => toJson().isEmpty;

  Json toJson() => {
        if (preferredLanguage != null) 'preferred_language': preferredLanguage,
        if (uiLanguage != null) 'ui_language': uiLanguage,
        if (deliveryChannel != null) 'delivery_channel': deliveryChannel!.wire,
        if (deliverySlot != null) 'delivery_slot': deliverySlot!.wire,
        if (captionsOn != null) 'captions_on': captionsOn,
        if (remindersOn != null) 'reminders_on': remindersOn,
        if (extraReelsOn != null) 'extra_reels_on': extraReelsOn,
        if (clearPausedUntil)
          'paused_until': null
        else if (pausedUntil != null)
          'paused_until': pausedUntil,
        if (pushToken != null) 'push_token': pushToken,
      };
}

class TokenPair {
  const TokenPair({
    required this.accessToken,
    required this.refreshToken,
    required this.expiresIn,
    required this.user,
  });

  factory TokenPair.fromJson(Json j) => TokenPair(
        accessToken: asString(j, 'access_token'),
        refreshToken: asString(j, 'refresh_token'),
        expiresIn: asInt(j, 'expires_in', fallback: 900),
        user: Me.fromJson(asJsonOrNull(j, 'user') ?? const {}),
      );

  final String accessToken;
  final String refreshToken;
  final int expiresIn;
  final Me user;

  Json toJson() => {
        'access_token': accessToken,
        'refresh_token': refreshToken,
        'expires_in': expiresIn,
        'user': user.toJson(),
      };
}

class PrivacyStatement {
  const PrivacyStatement({
    required this.managerSees,
    required this.hrSees,
    required this.nobodySees,
  });

  factory PrivacyStatement.fromJson(Json j) => PrivacyStatement(
        managerSees: asStringList(j, 'manager_sees'),
        hrSees: asStringList(j, 'hr_sees'),
        nobodySees: asStringList(j, 'nobody_sees'),
      );

  final List<String> managerSees;
  final List<String> hrSees;
  final List<String> nobodySees;

  Json toJson() => {
        'manager_sees': managerSees,
        'hr_sees': hrSees,
        'nobody_sees': nobodySees,
      };
}

class Export {
  const Export({
    required this.id,
    required this.kind,
    required this.status,
    required this.createdAt,
  });

  factory Export.fromJson(Json j) => Export(
        id: asString(j, 'id'),
        kind: asString(j, 'kind'),
        status: asString(j, 'status'),
        createdAt: asString(j, 'created_at'),
      );

  final String id;
  final String kind;
  final String status;
  final String createdAt;

  Json toJson() => {
        'id': id,
        'kind': kind,
        'status': status,
        'created_at': createdAt,
      };
}
