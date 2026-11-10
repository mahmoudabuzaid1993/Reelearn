import 'enums.dart';
import 'json.dart';
import 'learn_models.dart';

/// Signals the app sends to `POST /me/events`. Completions go through
/// `/me/watch`, saves through `/me/saved`, shares through `/me/shares`.
enum EngagementKind {
  start('start'),
  skip('skip'),
  rewatch('rewatch'),
  notRelevant('not_relevant'),
  moreLikeThis('more_like_this');

  const EngagementKind(this.wire);
  final String wire;

  static EngagementKind parse(Object? v) =>
      values.firstWhere((e) => e.wire == v, orElse: () => EngagementKind.start);
}

/// Leaving a reel before this counts as a quick skip.
const skipThresholdMs = 3000;

class EngagementSignal {
  const EngagementSignal({
    required this.reelId,
    required this.kind,
    this.watchedMs,
    this.position,
  });

  factory EngagementSignal.fromJson(Json j) => EngagementSignal(
        reelId: asString(j, 'reel_id'),
        kind: EngagementKind.parse(j['kind']),
        watchedMs: asIntOrNull(j, 'watched_ms'),
        position: asIntOrNull(j, 'position'),
      );

  final String reelId;
  final EngagementKind kind;
  final int? watchedMs;
  final int? position;

  Json toJson() => {
        'reel_id': reelId,
        'kind': kind.wire,
        if (watchedMs != null) 'watched_ms': watchedMs,
        if (position != null) 'position': position,
      };
}

/// Body of `POST /me/events`.
class EngagementBatch {
  const EngagementBatch({
    this.sessionId,
    this.activeMs,
    this.events = const [],
  });

  factory EngagementBatch.fromJson(Json j) => EngagementBatch(
        sessionId: asStringOrNull(j, 'session_id'),
        activeMs: asIntOrNull(j, 'active_ms'),
        events: asJsonList(j, 'events')
            .map(EngagementSignal.fromJson)
            .toList(growable: false),
      );

  final String? sessionId;
  final int? activeMs;
  final List<EngagementSignal> events;

  Json toJson() => {
        if (sessionId != null) 'session_id': sessionId,
        if (activeMs != null) 'active_ms': activeMs,
        'events': [for (final e in events) e.toJson()],
      };
}

enum PracticeStop {
  disabled('disabled'),
  dailyCap('daily_cap'),
  sessionCap('session_cap'),
  outsideWorkHours('outside_work_hours'),
  paused('paused'),
  exhausted('exhausted');

  const PracticeStop(this.wire);
  final String wire;

  static PracticeStop? parse(Object? v) =>
      values.where((e) => e.wire == v).firstOrNull;
}

class PracticeBudget {
  const PracticeBudget({
    required this.open,
    this.stop,
    this.remainingTodayS,
    this.remainingSessionS,
    this.breakEveryS,
  });

  factory PracticeBudget.fromJson(Json j) => PracticeBudget(
        open: asBool(j, 'open'),
        stop: PracticeStop.parse(j['stop']),
        remainingTodayS: asIntOrNull(j, 'remaining_today_s'),
        remainingSessionS: asIntOrNull(j, 'remaining_session_s'),
        breakEveryS: asIntOrNull(j, 'break_every_s'),
      );

  final bool open;
  final PracticeStop? stop;
  final int? remainingTodayS;
  final int? remainingSessionS;
  final int? breakEveryS;
}

/// One ranked item of a practice batch. Reasons: shared, mandatory, followed,
/// interest, popular, new, explore, assigned, catalog, review.
class NextItem {
  const NextItem({
    required this.id,
    required this.position,
    required this.kind,
    required this.reason,
    this.reasonDetail,
    this.reel,
    this.question,
    this.reviewNote,
  });

  factory NextItem.fromJson(Json j) => NextItem(
        id: asString(j, 'id'),
        position: asInt(j, 'position'),
        kind: PlanItemKind.parse(j['kind']),
        reason: asString(j, 'reason'),
        reasonDetail: asStringOrNull(j, 'reason_detail'),
        reel: asJsonOrNull(j, 'reel') == null
            ? null
            : ReelPlayable.fromJson(asJsonOrNull(j, 'reel')!),
        question: asJsonOrNull(j, 'question') == null
            ? null
            : Question.fromJson(asJsonOrNull(j, 'question')!),
        reviewNote: asStringOrNull(j, 'review_note'),
      );

  final String id;
  final int position;
  final PlanItemKind kind;
  final String reason;
  final String? reasonDetail;
  final ReelPlayable? reel;
  final Question? question;
  final String? reviewNote;

  /// The swipe stack plays plan items; a practice item is one that is not in
  /// today's plan, so its watch and answer carry no plan item id.
  PlanItem toPlanItem() => PlanItem(
        id: id,
        position: position,
        kind: kind,
        reason:
            reason == 'review' ? PlanItemReason.review : PlanItemReason.isNew,
        state: PlanItemState.pending,
        reel: reel,
        question: question,
        reviewNote: reviewNote,
        inPlan: false,
        practiceReason: reason,
        reasonDetail: reasonDetail,
      );
}

class NextBatch {
  const NextBatch({
    required this.sessionId,
    required this.items,
    required this.budget,
    this.autoplayNext = true,
    this.peerSharing = true,
  });

  factory NextBatch.fromJson(Json j) => NextBatch(
        sessionId: asString(j, 'session_id'),
        items: asJsonList(j, 'items')
            .map(NextItem.fromJson)
            .toList(growable: false),
        budget: PracticeBudget.fromJson(asJsonOrNull(j, 'budget') ?? const {}),
        autoplayNext: asBool(j, 'autoplay_next', fallback: true),
        peerSharing: asBool(j, 'peer_sharing', fallback: true),
      );

  final String sessionId;
  final List<NextItem> items;
  final PracticeBudget budget;
  final bool autoplayNext;
  final bool peerSharing;
}

enum TopicPreference {
  follow('follow'),
  mute('mute');

  const TopicPreference(this.wire);
  final String wire;

  static TopicPreference? parse(Object? v) =>
      values.where((e) => e.wire == v).firstOrNull;
}

class MyTopic {
  const MyTopic({
    required this.topicId,
    required this.name,
    required this.isMandatory,
    required this.assigned,
    required this.interest,
    required this.publishedReels,
    this.preference,
  });

  factory MyTopic.fromJson(Json j) => MyTopic(
        topicId: asString(j, 'topic_id'),
        name: asString(j, 'name'),
        isMandatory: asBool(j, 'is_mandatory'),
        assigned: asBool(j, 'assigned'),
        preference: TopicPreference.parse(j['preference']),
        interest: asInt(j, 'interest'),
        publishedReels: asInt(j, 'published_reels'),
      );

  final String topicId;
  final String name;
  final bool isMandatory;
  final bool assigned;
  final TopicPreference? preference;

  /// 0..100.
  final int interest;
  final int publishedReels;

  MyTopic copyWith({TopicPreference? preference, bool clear = false}) =>
      MyTopic(
        topicId: topicId,
        name: name,
        isMandatory: isMandatory,
        assigned: assigned,
        preference: clear ? null : (preference ?? this.preference),
        interest: interest,
        publishedReels: publishedReels,
      );
}

class TagInterest {
  const TagInterest({required this.tag, required this.interest});

  factory TagInterest.fromJson(Json j) =>
      TagInterest(tag: asString(j, 'tag'), interest: asInt(j, 'interest'));

  final String tag;
  final int interest;
}

class MyInterests {
  const MyInterests({
    required this.personalized,
    required this.topics,
    required this.topTags,
  });

  factory MyInterests.fromJson(Json j) => MyInterests(
        personalized: asBool(j, 'personalized', fallback: true),
        topics: asJsonList(j, 'topics')
            .map(MyTopic.fromJson)
            .toList(growable: false),
        topTags: asJsonList(j, 'top_tags')
            .map(TagInterest.fromJson)
            .toList(growable: false),
      );

  final bool personalized;
  final List<MyTopic> topics;
  final List<TagInterest> topTags;
}

class Colleague {
  const Colleague({
    required this.id,
    required this.fullName,
    this.jobTitle,
    this.location,
  });

  factory Colleague.fromJson(Json j) => Colleague(
        id: asString(j, 'id'),
        fullName: asString(j, 'full_name'),
        jobTitle: asStringOrNull(j, 'job_title'),
        location: asStringOrNull(j, 'location'),
      );

  final String id;
  final String fullName;
  final String? jobTitle;
  final String? location;
}
