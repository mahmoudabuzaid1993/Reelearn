import 'enums.dart';
import 'json.dart';

class TeamTopic {
  const TeamTopic({
    required this.id,
    required this.name,
    required this.readyCount,
  });

  factory TeamTopic.fromJson(Json j) => TeamTopic(
        id: asString(j, 'id'),
        name: asString(j, 'name'),
        readyCount: asInt(j, 'ready_count'),
      );

  final String id;
  final String name;
  final int readyCount;

  Json toJson() => {'id': id, 'name': name, 'ready_count': readyCount};
}

class MemberReadiness {
  const MemberReadiness({required this.topicId, required this.state});

  factory MemberReadiness.fromJson(Json j) => MemberReadiness(
        topicId: asString(j, 'topic_id'),
        state: ReadinessState.parse(j['state']),
      );

  final String topicId;
  final ReadinessState state;

  Json toJson() => {'topic_id': topicId, 'state': state.wire};
}

class TeamMember {
  const TeamMember({
    required this.userId,
    required this.fullName,
    required this.readiness,
    this.language = 'en',
    this.lastActiveOn,
    this.nudgedToday = false,
  });

  factory TeamMember.fromJson(Json j) => TeamMember(
        userId: asString(j, 'user_id'),
        fullName: asString(j, 'full_name'),
        language: asString(j, 'language', fallback: 'en'),
        lastActiveOn: asStringOrNull(j, 'last_active_on'),
        nudgedToday: asBool(j, 'nudged_today'),
        readiness: asJsonList(j, 'readiness')
            .map(MemberReadiness.fromJson)
            .toList(growable: false),
      );

  final String userId;
  final String fullName;
  final String language;
  final String? lastActiveOn;
  final bool nudgedToday;
  final List<MemberReadiness> readiness;

  ReadinessState stateFor(String topicId) => readiness
      .firstWhere(
        (r) => r.topicId == topicId,
        orElse: () =>
            MemberReadiness(topicId: topicId, state: ReadinessState.notStarted),
      )
      .state;

  TeamMember copyWith({bool? nudgedToday}) => TeamMember(
        userId: userId,
        fullName: fullName,
        language: language,
        lastActiveOn: lastActiveOn,
        nudgedToday: nudgedToday ?? this.nudgedToday,
        readiness: readiness,
      );

  Json toJson() => {
        'user_id': userId,
        'full_name': fullName,
        'language': language,
        'last_active_on': lastActiveOn,
        'nudged_today': nudgedToday,
        'readiness': readiness.map((r) => r.toJson()).toList(),
      };
}

class TeamReadiness {
  const TeamReadiness({
    required this.teamRecallPct,
    required this.topics,
    required this.members,
    this.activeThisWeek = 0,
    this.teamSize = 0,
  });

  factory TeamReadiness.fromJson(Json j) => TeamReadiness(
        teamRecallPct: asInt(j, 'team_recall_pct'),
        activeThisWeek: asInt(j, 'active_this_week'),
        teamSize: asInt(j, 'team_size'),
        topics: asJsonList(j, 'topics')
            .map(TeamTopic.fromJson)
            .toList(growable: false),
        members: asJsonList(j, 'members')
            .map(TeamMember.fromJson)
            .toList(growable: false),
      );

  final int teamRecallPct;
  final int activeThisWeek;
  final int teamSize;
  final List<TeamTopic> topics;
  final List<TeamMember> members;

  TeamReadiness copyWith({List<TeamMember>? members}) => TeamReadiness(
        teamRecallPct: teamRecallPct,
        activeThisWeek: activeThisWeek,
        teamSize: teamSize,
        topics: topics,
        members: members ?? this.members,
      );

  Json toJson() => {
        'team_recall_pct': teamRecallPct,
        'active_this_week': activeThisWeek,
        'team_size': teamSize,
        'topics': topics.map((t) => t.toJson()).toList(),
        'members': members.map((m) => m.toJson()).toList(),
      };
}

class NudgeResult {
  const NudgeResult({required this.queued, required this.skippedAlreadyNudged});

  factory NudgeResult.fromJson(Json j) => NudgeResult(
        queued: asInt(j, 'queued'),
        skippedAlreadyNudged: asInt(j, 'skipped_already_nudged'),
      );

  final int queued;
  final int skippedAlreadyNudged;

  Json toJson() =>
      {'queued': queued, 'skipped_already_nudged': skippedAlreadyNudged};
}

class ReelRequest {
  const ReelRequest({
    required this.id,
    required this.text,
    this.votes = 0,
    this.status = 'open',
  });

  factory ReelRequest.fromJson(Json j) => ReelRequest(
        id: asString(j, 'id'),
        text: asString(j, 'text'),
        votes: asInt(j, 'votes'),
        status: asString(j, 'status', fallback: 'open'),
      );

  final String id;
  final String text;
  final int votes;
  final String status;

  Json toJson() => {'id': id, 'text': text, 'votes': votes, 'status': status};
}

class UploadSlot {
  const UploadSlot({
    required this.id,
    required this.uploadUrl,
    required this.method,
    required this.headers,
    required this.expiresAt,
  });

  factory UploadSlot.fromJson(Json j) {
    final raw = asJsonOrNull(j, 'headers') ?? const {};
    return UploadSlot(
      id: asString(j, 'id'),
      uploadUrl: asString(j, 'upload_url'),
      method: asString(j, 'method', fallback: 'PUT'),
      headers: {
        for (final e in raw.entries)
          if (e.value is String) e.key: e.value as String,
      },
      expiresAt: asString(j, 'expires_at'),
    );
  }

  final String id;
  final String uploadUrl;
  final String method;
  final Map<String, String> headers;
  final String expiresAt;

  Json toJson() => {
        'id': id,
        'upload_url': uploadUrl,
        'method': method,
        'headers': headers,
        'expires_at': expiresAt,
      };
}

/// The subset of `Reel` the phone needs after submitting a recording.
class DraftReel {
  const DraftReel({
    required this.id,
    required this.title,
    required this.status,
  });

  factory DraftReel.fromJson(Json j) => DraftReel(
        id: asString(j, 'id'),
        title: asString(j, 'title'),
        status: asString(j, 'status', fallback: 'draft'),
      );

  final String id;
  final String title;
  final String status;

  Json toJson() => {'id': id, 'title': title, 'status': status};
}
