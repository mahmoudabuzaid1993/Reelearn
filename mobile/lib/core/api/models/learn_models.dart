import 'enums.dart';
import 'json.dart';

class QuestionOption {
  const QuestionOption({required this.key, required this.text});

  factory QuestionOption.fromJson(Json j) =>
      QuestionOption(key: asString(j, 'key'), text: asString(j, 'text'));

  final String key;
  final String text;

  Json toJson() => {'key': key, 'text': text};
}

class Question {
  const Question({
    required this.id,
    required this.language,
    required this.prompt,
    required this.options,
    this.correctKey,
    this.explanation,
  });

  factory Question.fromJson(Json j) => Question(
        id: asString(j, 'id'),
        language: asString(j, 'language', fallback: 'en'),
        prompt: asString(j, 'prompt'),
        options: asJsonList(j, 'options')
            .map(QuestionOption.fromJson)
            .toList(growable: false),
        correctKey: asStringOrNull(j, 'correct_key'),
        explanation: asStringOrNull(j, 'explanation'),
      );

  final String id;
  final String language;
  final String prompt;
  final List<QuestionOption> options;

  /// Never present in learner payloads; kept for completeness of the contract.
  final String? correctKey;
  final String? explanation;

  Json toJson() => {
        'id': id,
        'language': language,
        'prompt': prompt,
        'options': options.map((o) => o.toJson()).toList(),
        if (correctKey != null) 'correct_key': correctKey,
        'explanation': explanation,
      };
}

class ReelPlayable {
  const ReelPlayable({
    required this.id,
    required this.title,
    required this.topicName,
    required this.language,
    required this.durationMs,
    required this.videoUrl,
    required this.captionsOnByDefault,
    this.presenterName,
    this.captionsUrl,
    this.thumbnailUrl,
    this.captionText,
    this.sourceCitation,
    this.whyAmISeeingThis = '',
    this.question,
    this.saved = false,
  });

  factory ReelPlayable.fromJson(Json j) => ReelPlayable(
        id: asString(j, 'id'),
        title: asString(j, 'title'),
        topicName: asString(j, 'topic_name'),
        presenterName: asStringOrNull(j, 'presenter_name'),
        language: asString(j, 'language', fallback: 'en'),
        durationMs: asInt(j, 'duration_ms'),
        videoUrl: asString(j, 'video_url'),
        captionsUrl: asStringOrNull(j, 'captions_url'),
        thumbnailUrl: asStringOrNull(j, 'thumbnail_url'),
        captionsOnByDefault:
            asBool(j, 'captions_on_by_default', fallback: true),
        captionText: asStringOrNull(j, 'caption_text'),
        sourceCitation: asStringOrNull(j, 'source_citation'),
        whyAmISeeingThis: asString(j, 'why_am_i_seeing_this'),
        question: asJsonOrNull(j, 'question') == null
            ? null
            : Question.fromJson(asJsonOrNull(j, 'question')!),
        saved: asBool(j, 'saved'),
      );

  final String id;
  final String title;
  final String topicName;
  final String? presenterName;
  final String language;
  final int durationMs;
  final String videoUrl;
  final String? captionsUrl;
  final String? thumbnailUrl;
  final bool captionsOnByDefault;
  final String? captionText;
  final String? sourceCitation;
  final String whyAmISeeingThis;
  final Question? question;
  final bool saved;

  ReelPlayable copyWith({bool? saved}) => ReelPlayable(
        id: id,
        title: title,
        topicName: topicName,
        presenterName: presenterName,
        language: language,
        durationMs: durationMs,
        videoUrl: videoUrl,
        captionsUrl: captionsUrl,
        thumbnailUrl: thumbnailUrl,
        captionsOnByDefault: captionsOnByDefault,
        captionText: captionText,
        sourceCitation: sourceCitation,
        whyAmISeeingThis: whyAmISeeingThis,
        question: question,
        saved: saved ?? this.saved,
      );

  Json toJson() => {
        'id': id,
        'title': title,
        'topic_name': topicName,
        'presenter_name': presenterName,
        'language': language,
        'duration_ms': durationMs,
        'video_url': videoUrl,
        'captions_url': captionsUrl,
        'thumbnail_url': thumbnailUrl,
        'captions_on_by_default': captionsOnByDefault,
        'caption_text': captionText,
        'source_citation': sourceCitation,
        'why_am_i_seeing_this': whyAmISeeingThis,
        'question': question?.toJson(),
        'saved': saved,
      };
}

class PlanItem {
  const PlanItem({
    required this.id,
    required this.position,
    required this.kind,
    required this.reason,
    required this.state,
    this.reel,
    this.question,
    this.reviewNote,
    this.inPlan = true,
    this.practiceReason,
    this.reasonDetail,
  });

  factory PlanItem.fromJson(Json j) => PlanItem(
        id: asString(j, 'id'),
        position: asInt(j, 'position'),
        kind: PlanItemKind.parse(j['kind']),
        reason: PlanItemReason.parse(j['reason']),
        state: PlanItemState.parse(j['state']),
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
  final PlanItemReason reason;
  final PlanItemState state;
  final ReelPlayable? reel;
  final Question? question;
  final String? reviewNote;

  /// False for practice items (`/me/next`): they are not rows of today's plan,
  /// so watch and answer events carry no plan item id.
  final bool inPlan;

  /// Why the engine chose a practice item (shared, interest, popular, ...).
  final String? practiceReason;

  /// The topic, the colleague's first name or the team for the reason line.
  final String? reasonDetail;

  /// The id to send as `plan_item_id`, or null for practice items.
  String? get planItemId => inPlan ? id : null;

  /// The question for this item, whichever field the API filled.
  Question? get effectiveQuestion => question ?? reel?.question;

  PlanItem copyWith({PlanItemState? state, ReelPlayable? reel}) => PlanItem(
        id: id,
        position: position,
        kind: kind,
        reason: reason,
        state: state ?? this.state,
        reel: reel ?? this.reel,
        question: question,
        reviewNote: reviewNote,
        inPlan: inPlan,
        practiceReason: practiceReason,
        reasonDetail: reasonDetail,
      );

  Json toJson() => {
        'id': id,
        'position': position,
        'kind': kind.wire,
        'reason': reason.wire,
        'state': state.wire,
        'reel': reel?.toJson(),
        'question': question?.toJson(),
        'review_note': reviewNote,
      };
}

class UserStats {
  const UserStats({
    required this.streakCurrent,
    required this.streakLongest,
    required this.freezesAvailable,
    required this.rememberedCount,
    this.reelsCompleted = 0,
    this.lastActiveOn,
  });

  factory UserStats.fromJson(Json j) => UserStats(
        streakCurrent: asInt(j, 'streak_current'),
        streakLongest: asInt(j, 'streak_longest'),
        freezesAvailable: asInt(j, 'freezes_available'),
        rememberedCount: asInt(j, 'remembered_count'),
        reelsCompleted: asInt(j, 'reels_completed'),
        lastActiveOn: asStringOrNull(j, 'last_active_on'),
      );

  static const empty = UserStats(
    streakCurrent: 0,
    streakLongest: 0,
    freezesAvailable: 0,
    rememberedCount: 0,
  );

  final int streakCurrent;
  final int streakLongest;
  final int freezesAvailable;
  final int rememberedCount;
  final int reelsCompleted;
  final String? lastActiveOn;

  UserStats copyWith({
    int? streakCurrent,
    int? streakLongest,
    int? freezesAvailable,
    int? rememberedCount,
    int? reelsCompleted,
    String? lastActiveOn,
  }) =>
      UserStats(
        streakCurrent: streakCurrent ?? this.streakCurrent,
        streakLongest: streakLongest ?? this.streakLongest,
        freezesAvailable: freezesAvailable ?? this.freezesAvailable,
        rememberedCount: rememberedCount ?? this.rememberedCount,
        reelsCompleted: reelsCompleted ?? this.reelsCompleted,
        lastActiveOn: lastActiveOn ?? this.lastActiveOn,
      );

  Json toJson() => {
        'streak_current': streakCurrent,
        'streak_longest': streakLongest,
        'freezes_available': freezesAvailable,
        'remembered_count': rememberedCount,
        'reels_completed': reelsCompleted,
        'last_active_on': lastActiveOn,
      };
}

class Yesterday {
  const Yesterday({
    required this.remembered,
    required this.total,
    this.nextReviewOn,
  });

  factory Yesterday.fromJson(Json j) => Yesterday(
        remembered: asInt(j, 'remembered'),
        total: asInt(j, 'total'),
        nextReviewOn: asStringOrNull(j, 'next_review_on'),
      );

  final int remembered;
  final int total;
  final String? nextReviewOn;

  Json toJson() => {
        'remembered': remembered,
        'total': total,
        'next_review_on': nextReviewOn,
      };
}

class DailyPlan {
  const DailyPlan({
    required this.id,
    required this.planDate,
    required this.status,
    required this.items,
    required this.stats,
    this.greetingName,
    this.estimatedSeconds = 0,
    this.yesterday,
    this.practiceAvailable = false,
  });

  factory DailyPlan.fromJson(Json j) => DailyPlan(
        id: asString(j, 'id'),
        planDate: asString(j, 'plan_date'),
        status: PlanStatus.parse(j['status']),
        greetingName: asStringOrNull(j, 'greeting_name'),
        estimatedSeconds: asInt(j, 'estimated_seconds'),
        items: asJsonList(j, 'items')
            .map(PlanItem.fromJson)
            .toList(growable: false),
        stats: UserStats.fromJson(asJsonOrNull(j, 'stats') ?? const {}),
        yesterday: asJsonOrNull(j, 'yesterday') == null
            ? null
            : Yesterday.fromJson(asJsonOrNull(j, 'yesterday')!),
        practiceAvailable: asBool(j, 'practice_available'),
      );

  final String id;
  final String planDate;
  final PlanStatus status;
  final String? greetingName;
  final int estimatedSeconds;
  final List<PlanItem> items;
  final UserStats stats;
  final Yesterday? yesterday;

  /// The company offers "keep going" practice after the minute.
  final bool practiceAvailable;

  int get newReels => items
      .where(
        (i) => i.kind == PlanItemKind.reel && i.reason == PlanItemReason.isNew,
      )
      .length;
  int get reviews =>
      items.where((i) => i.reason == PlanItemReason.review).length;
  bool get isEmpty => status == PlanStatus.empty || items.isEmpty;
  bool get isDone => status == PlanStatus.done;

  DailyPlan copyWith({
    PlanStatus? status,
    List<PlanItem>? items,
    UserStats? stats,
  }) =>
      DailyPlan(
        id: id,
        planDate: planDate,
        status: status ?? this.status,
        greetingName: greetingName,
        estimatedSeconds: estimatedSeconds,
        items: items ?? this.items,
        stats: stats ?? this.stats,
        yesterday: yesterday,
        practiceAvailable: practiceAvailable,
      );

  Json toJson() => {
        'id': id,
        'plan_date': planDate,
        'status': status.wire,
        'greeting_name': greetingName,
        'estimated_seconds': estimatedSeconds,
        'items': items.map((i) => i.toJson()).toList(),
        'stats': stats.toJson(),
        'yesterday': yesterday?.toJson(),
        'practice_available': practiceAvailable,
      };
}

class AnswerOutcome {
  const AnswerOutcome({
    required this.isCorrect,
    required this.correctKey,
    required this.nextDueOn,
    required this.planStatus,
    this.explanation,
    this.stats,
  });

  factory AnswerOutcome.fromJson(Json j) => AnswerOutcome(
        isCorrect: asBool(j, 'is_correct'),
        correctKey: asString(j, 'correct_key'),
        explanation: asStringOrNull(j, 'explanation'),
        nextDueOn: asString(j, 'next_due_on'),
        planStatus: PlanStatus.parse(j['plan_status']),
        stats: asJsonOrNull(j, 'stats') == null
            ? null
            : UserStats.fromJson(asJsonOrNull(j, 'stats')!),
      );

  final bool isCorrect;
  final String correctKey;
  final String? explanation;
  final String nextDueOn;
  final PlanStatus planStatus;
  final UserStats? stats;

  Json toJson() => {
        'is_correct': isCorrect,
        'correct_key': correctKey,
        'explanation': explanation,
        'next_due_on': nextDueOn,
        'plan_status': planStatus.wire,
        'stats': stats?.toJson(),
      };
}

class WeekPoint {
  const WeekPoint({required this.week, required this.rememberedPct});

  factory WeekPoint.fromJson(Json j) => WeekPoint(
        week: asString(j, 'week'),
        rememberedPct: asInt(j, 'remembered_pct'),
      );

  final String week;
  final int rememberedPct;

  Json toJson() => {'week': week, 'remembered_pct': rememberedPct};
}

class TopicProgress {
  const TopicProgress({
    required this.topicId,
    required this.name,
    required this.recallPct,
    this.nextReviewOn,
  });

  factory TopicProgress.fromJson(Json j) => TopicProgress(
        topicId: asString(j, 'topic_id'),
        name: asString(j, 'name'),
        recallPct: asInt(j, 'recall_pct'),
        nextReviewOn: asStringOrNull(j, 'next_review_on'),
      );

  final String topicId;
  final String name;
  final int recallPct;
  final String? nextReviewOn;

  Json toJson() => {
        'topic_id': topicId,
        'name': name,
        'recall_pct': recallPct,
        'next_review_on': nextReviewOn,
      };
}

class Milestone {
  const Milestone({
    required this.key,
    required this.label,
    required this.reached,
  });

  factory Milestone.fromJson(Json j) => Milestone(
        key: asString(j, 'key'),
        label: asString(j, 'label'),
        reached: asBool(j, 'reached'),
      );

  final String key;
  final String label;
  final bool reached;

  Json toJson() => {'key': key, 'label': label, 'reached': reached};
}

class Progress {
  const Progress({
    required this.rememberedPct,
    required this.series,
    required this.byTopic,
    required this.milestones,
    required this.stats,
  });

  factory Progress.fromJson(Json j) => Progress(
        rememberedPct: asInt(j, 'remembered_pct'),
        series: asJsonList(j, 'series')
            .map(WeekPoint.fromJson)
            .toList(growable: false),
        byTopic: asJsonList(j, 'by_topic')
            .map(TopicProgress.fromJson)
            .toList(growable: false),
        milestones: asJsonList(j, 'milestones')
            .map(Milestone.fromJson)
            .toList(growable: false),
        stats: UserStats.fromJson(asJsonOrNull(j, 'stats') ?? const {}),
      );

  final int rememberedPct;
  final List<WeekPoint> series;
  final List<TopicProgress> byTopic;
  final List<Milestone> milestones;
  final UserStats stats;

  Json toJson() => {
        'remembered_pct': rememberedPct,
        'series': series.map((s) => s.toJson()).toList(),
        'by_topic': byTopic.map((t) => t.toJson()).toList(),
        'milestones': milestones.map((m) => m.toJson()).toList(),
        'stats': stats.toJson(),
      };
}

class ReelCard {
  const ReelCard({
    required this.id,
    required this.title,
    required this.topicName,
    required this.durationMs,
    required this.language,
    required this.saved,
    this.presenterName,
    this.thumbnailUrl,
    this.seen = false,
  });

  factory ReelCard.fromJson(Json j) => ReelCard(
        id: asString(j, 'id'),
        title: asString(j, 'title'),
        topicName: asString(j, 'topic_name'),
        presenterName: asStringOrNull(j, 'presenter_name'),
        durationMs: asInt(j, 'duration_ms'),
        language: asString(j, 'language', fallback: 'en'),
        thumbnailUrl: asStringOrNull(j, 'thumbnail_url'),
        saved: asBool(j, 'saved'),
        seen: asBool(j, 'seen'),
      );

  final String id;
  final String title;
  final String topicName;
  final String? presenterName;
  final int durationMs;
  final String language;
  final String? thumbnailUrl;
  final bool saved;
  final bool seen;

  ReelCard copyWith({bool? saved}) => ReelCard(
        id: id,
        title: title,
        topicName: topicName,
        presenterName: presenterName,
        durationMs: durationMs,
        language: language,
        thumbnailUrl: thumbnailUrl,
        saved: saved ?? this.saved,
        seen: seen,
      );

  Json toJson() => {
        'id': id,
        'title': title,
        'topic_name': topicName,
        'presenter_name': presenterName,
        'duration_ms': durationMs,
        'language': language,
        'thumbnail_url': thumbnailUrl,
        'saved': saved,
        'seen': seen,
      };
}

/// Cursor page. `items` is parsed by the caller-provided mapper.
class Page<T> {
  const Page({required this.items, this.nextCursor});

  factory Page.fromJson(Json j, T Function(Json) mapItem) => Page(
        items: asJsonList(j, 'items').map(mapItem).toList(growable: false),
        nextCursor: asStringOrNull(j, 'next_cursor'),
      );

  final List<T> items;
  final String? nextCursor;
}
