import 'models/models.dart';

/// Demo data matching the API seed: tenant Al Wadi Retail, learner Sara,
/// manager Khalid. Shared by [FakeApi] and the tests.
class Fixtures {
  Fixtures._();

  static const tenantId = '0f3a3c2e-0000-4000-8000-00000000a1a1';
  static const tenant = TenantPublic(
    id: tenantId,
    slug: 'alwadi',
    name: 'Al Wadi Retail',
    languages: ['ar', 'en', 'ur', 'bn'],
  );

  static const saraId = '0f3a3c2e-0000-4000-8000-000000000010';
  static const khalidId = '0f3a3c2e-0000-4000-8000-000000000002';
  static const rizwanId = '0f3a3c2e-0000-4000-8000-000000000011';
  static const mohammadId = '0f3a3c2e-0000-4000-8000-000000000012';
  static const fatimahId = '0f3a3c2e-0000-4000-8000-000000000013';

  static const saraPhone = '+966500000010';
  static const khalidPhone = '+966500000002';

  static const sara = Me(
    id: saraId,
    tenant: tenant,
    role: Role.learner,
    fullName: 'Sara Al-Qahtani',
    firstName: 'Sara',
    phoneE164: saraPhone,
    preferredLanguage: 'ar',
    uiLanguage: 'ar',
    deliveryChannel: Channel.app,
    deliverySlot: DeliverySlot.shiftStart,
    isFirstOpen: true,
  );

  static const khalid = Me(
    id: khalidId,
    tenant: tenant,
    role: Role.manager,
    fullName: 'Khalid Mansour',
    firstName: 'Khalid',
    phoneE164: khalidPhone,
    preferredLanguage: 'en',
    uiLanguage: 'en',
    deliveryChannel: Channel.app,
    deliverySlot: DeliverySlot.shiftStart,
  );

  static const topicCustomerService = '0f3a3c2e-0000-4000-8000-0000000000c1';
  static const topicStoreSafety = '0f3a3c2e-0000-4000-8000-0000000000c2';
  static const topicAml = '0f3a3c2e-0000-4000-8000-0000000000c3';

  static const planId = '0f3a3c2e-0000-4000-8000-0000000000p1';
  static const item1Id = '0f3a3c2e-0000-4000-8000-000000000i01';
  static const item2Id = '0f3a3c2e-0000-4000-8000-000000000i02';
  static const item3Id = '0f3a3c2e-0000-4000-8000-000000000i03';

  static const reel1Id = '0f3a3c2e-0000-4000-8000-0000000000r1';
  static const reel2Id = '0f3a3c2e-0000-4000-8000-0000000000r2';
  static const question1Id = '0f3a3c2e-0000-4000-8000-0000000000q1';
  static const question2Id = '0f3a3c2e-0000-4000-8000-0000000000q2';
  static const question3Id = '0f3a3c2e-0000-4000-8000-0000000000q3';

  /// Correct keys live here, never in the learner payloads.
  static const correctKeys = {
    question1Id: 'B',
    question2Id: 'B',
    question3Id: 'A',
  };

  static const explanations = {
    question1Id: 'Twice is enough; more sounds like a script.',
    question2Id:
        'The cone warns people before anyone slips. The mop comes second.',
    question3Id: 'SAR 60,000 is the threshold. Report it the same day.',
  };

  static const question1 = Question(
    id: question1Id,
    language: 'en',
    prompt: 'A customer, Fatimah, is paying. When do you use her name?',
    options: [
      QuestionOption(key: 'A', text: 'Only at checkout'),
      QuestionOption(key: 'B', text: 'Once at hello, once at goodbye'),
      QuestionOption(key: 'C', text: 'Every sentence, to be warm'),
    ],
  );

  static const question2 = Question(
    id: question2Id,
    language: 'en',
    prompt: 'A bottle breaks in aisle 4. What is your first move?',
    options: [
      QuestionOption(key: 'A', text: 'Mop it up quickly'),
      QuestionOption(key: 'B', text: 'Cone first, then mop'),
      QuestionOption(key: 'C', text: 'Call a colleague and wait'),
    ],
  );

  static const question3 = Question(
    id: question3Id,
    language: 'en',
    prompt: 'Which cash transactions must be reported?',
    options: [
      QuestionOption(key: 'A', text: 'Any cash of SAR 60,000 or more'),
      QuestionOption(key: 'B', text: 'Only transfers abroad'),
      QuestionOption(key: 'C', text: 'Only amounts over SAR 100,000'),
    ],
  );

  /// Stand-in media for the fake API. `--dart-define=SAMPLE_VIDEO=…` (and `_2`)
  /// point the two demo reels at local clips.
  static const sampleVideo = String.fromEnvironment(
    'SAMPLE_VIDEO',
    defaultValue:
        'https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ForBiggerEscapes.mp4',
  );
  static const sampleVideo2 = String.fromEnvironment(
    'SAMPLE_VIDEO_2',
    defaultValue: sampleVideo,
  );

  static const reel1 = ReelPlayable(
    id: reel1Id,
    title: "Say the customer's name twice in the first minute",
    topicName: 'Customer service',
    presenterName: 'Maha',
    language: 'en',
    durationMs: 45000,
    videoUrl: sampleVideo,
    captionsOnByDefault: true,
    captionText: 'Once at hello, once at goodbye. That is the whole trick.',
    sourceCitation: 'Al Wadi service standard, section 2.1',
    whyAmISeeingThis:
        'Topic: Customer service. Assigned to: Store floor. Source: Al Wadi service standard.',
    question: question1,
  );

  static const reel2 = ReelPlayable(
    id: reel2Id,
    title: 'Spill on the floor: cone first, mop second',
    topicName: 'Store safety',
    presenterName: 'Khalid',
    language: 'en',
    durationMs: 47000,
    videoUrl: sampleVideo2,
    captionsOnByDefault: true,
    captionText: 'Cone first. Nobody slips while you fetch the mop.',
    sourceCitation: 'Store safety handbook, chapter 3',
    whyAmISeeingThis:
        'Topic: Store safety. Assigned to: All stores. Source: Store safety handbook.',
    question: question2,
  );

  static const stats = UserStats(
    streakCurrent: 12,
    streakLongest: 20,
    freezesAvailable: 1,
    rememberedCount: 41,
    reelsCompleted: 58,
    lastActiveOn: '2026-10-29',
  );

  static const planItems = [
    PlanItem(
      id: item1Id,
      position: 1,
      kind: PlanItemKind.reel,
      reason: PlanItemReason.isNew,
      state: PlanItemState.pending,
      reel: reel1,
    ),
    PlanItem(
      id: item2Id,
      position: 2,
      kind: PlanItemKind.reel,
      reason: PlanItemReason.isNew,
      state: PlanItemState.pending,
      reel: reel2,
    ),
    PlanItem(
      id: item3Id,
      position: 3,
      kind: PlanItemKind.questionOnly,
      reason: PlanItemReason.review,
      state: PlanItemState.pending,
      question: question3,
      reviewNote: 'Asked again · day 10',
    ),
  ];

  static const plan = DailyPlan(
    id: planId,
    planDate: '2026-10-30',
    status: PlanStatus.open,
    greetingName: 'Sara',
    estimatedSeconds: 75,
    items: planItems,
    stats: stats,
    yesterday: Yesterday(remembered: 2, total: 3, nextReviewOn: '2026-11-02'),
  );

  static const progress = Progress(
    rememberedPct: 78,
    series: [
      WeekPoint(week: '2026-W37', rememberedPct: 52),
      WeekPoint(week: '2026-W38', rememberedPct: 58),
      WeekPoint(week: '2026-W39', rememberedPct: 61),
      WeekPoint(week: '2026-W40', rememberedPct: 66),
      WeekPoint(week: '2026-W41', rememberedPct: 70),
      WeekPoint(week: '2026-W42', rememberedPct: 72),
      WeekPoint(week: '2026-W43', rememberedPct: 75),
      WeekPoint(week: '2026-W44', rememberedPct: 78),
    ],
    byTopic: [
      TopicProgress(
        topicId: topicCustomerService,
        name: 'Customer service',
        recallPct: 85,
        nextReviewOn: '2026-11-02',
      ),
      TopicProgress(
        topicId: topicStoreSafety,
        name: 'Store safety',
        recallPct: 80,
        nextReviewOn: '2026-11-05',
      ),
      TopicProgress(
        topicId: topicAml,
        name: 'AML',
        recallPct: 64,
        nextReviewOn: '2026-10-31',
      ),
    ],
    milestones: [
      Milestone(key: 'first_week', label: 'First week', reached: true),
      Milestone(
        key: 'remembered_25',
        label: '25 things remembered',
        reached: true,
      ),
      Milestone(key: 'streak_30', label: '30-day streak', reached: false),
      Milestone(
        key: 'remembered_100',
        label: '100 things remembered',
        reached: false,
      ),
    ],
    stats: stats,
  );

  static const library = [
    ReelCard(
      id: reel1Id,
      title: "Say the customer's name twice in the first minute",
      topicName: 'Customer service',
      presenterName: 'Maha',
      durationMs: 45000,
      language: 'en',
      saved: true,
      seen: true,
    ),
    ReelCard(
      id: reel2Id,
      title: 'Spill on the floor: cone first, mop second',
      topicName: 'Store safety',
      presenterName: 'Khalid',
      durationMs: 47000,
      language: 'en',
      saved: false,
      seen: true,
    ),
    ReelCard(
      id: '0f3a3c2e-0000-4000-8000-0000000000r3',
      title: 'Greet within five seconds',
      topicName: 'Customer service',
      presenterName: 'Maha',
      durationMs: 38000,
      language: 'en',
      saved: false,
      seen: true,
    ),
    ReelCard(
      id: '0f3a3c2e-0000-4000-8000-0000000000r4',
      title: 'Fire exit: keep it clear, always',
      topicName: 'Store safety',
      presenterName: 'Khalid',
      durationMs: 41000,
      language: 'en',
      saved: true,
      seen: true,
    ),
    ReelCard(
      id: '0f3a3c2e-0000-4000-8000-0000000000r5',
      title: 'Cash over SAR 60,000: what to ask',
      topicName: 'AML',
      presenterName: 'Noura',
      durationMs: 52000,
      language: 'en',
      saved: false,
      seen: true,
    ),
    ReelCard(
      id: '0f3a3c2e-0000-4000-8000-0000000000r6',
      title: 'Return without a receipt',
      topicName: 'Customer service',
      presenterName: 'Maha',
      durationMs: 44000,
      language: 'en',
      saved: false,
    ),
  ];

  /// Reels the fake engine serves in practice ("keep going"), with a reason each.
  static List<({ReelPlayable reel, String reason, String? detail})>
      get practice => [
            for (final (i, c) in library.indexed.skip(2))
              (
                reel: ReelPlayable(
                  id: c.id,
                  title: c.title,
                  topicName: c.topicName,
                  presenterName: c.presenterName,
                  language: c.language,
                  durationMs: c.durationMs,
                  videoUrl: i.isEven ? sampleVideo : sampleVideo2,
                  captionsOnByDefault: true,
                  captionText: c.title,
                  whyAmISeeingThis: 'Topic: ${c.topicName}.',
                  question: i.isEven ? question1 : question2,
                ),
                reason: const [
                  'interest',
                  'popular',
                  'explore',
                  'assigned',
                ][i % 4],
                detail: i % 4 == 1 ? 'Jeddah' : c.topicName,
              ),
          ];

  static const colleagues = [
    Colleague(id: fatimahId, fullName: 'Fatimah Noor', location: 'Jeddah'),
    Colleague(id: rizwanId, fullName: 'Rizwan Ahmed', location: 'Jeddah'),
    Colleague(id: mohammadId, fullName: 'Mohammad A.', location: 'Jeddah'),
    Colleague(
      id: khalidId,
      fullName: 'Khalid Mansour',
      jobTitle: 'Store manager',
    ),
  ];

  static const interests = MyInterests(
    personalized: true,
    topics: [
      MyTopic(
        topicId: topicCustomerService,
        name: 'Customer service',
        isMandatory: false,
        assigned: true,
        interest: 72,
        publishedReels: 3,
        preference: TopicPreference.follow,
      ),
      MyTopic(
        topicId: topicStoreSafety,
        name: 'Store safety',
        isMandatory: true,
        assigned: true,
        interest: 41,
        publishedReels: 3,
      ),
      MyTopic(
        topicId: topicAml,
        name: 'AML',
        isMandatory: true,
        assigned: true,
        interest: 18,
        publishedReels: 2,
      ),
    ],
    topTags: [
      TagInterest(tag: 'greeting', interest: 64),
      TagInterest(tag: 'spills', interest: 38),
      TagInterest(tag: 'complaints', interest: 27),
    ],
  );

  static const topicIdsByName = {
    'Customer service': topicCustomerService,
    'Store safety': topicStoreSafety,
    'AML': topicAml,
  };

  static const team = TeamReadiness(
    teamRecallPct: 71,
    activeThisWeek: 3,
    teamSize: 4,
    topics: [
      TeamTopic(
        id: topicCustomerService,
        name: 'Customer service',
        readyCount: 2,
      ),
      TeamTopic(id: topicStoreSafety, name: 'Store safety', readyCount: 2),
      TeamTopic(id: topicAml, name: 'AML', readyCount: 1),
    ],
    members: [
      TeamMember(
        userId: saraId,
        fullName: 'Sara Al-Qahtani',
        language: 'ar',
        lastActiveOn: '2026-10-29',
        readiness: [
          MemberReadiness(
            topicId: topicCustomerService,
            state: ReadinessState.ready,
          ),
          MemberReadiness(
            topicId: topicStoreSafety,
            state: ReadinessState.ready,
          ),
          MemberReadiness(topicId: topicAml, state: ReadinessState.notYet),
        ],
      ),
      TeamMember(
        userId: rizwanId,
        fullName: 'Rizwan Ahmed',
        language: 'ur',
        lastActiveOn: '2026-10-27',
        readiness: [
          MemberReadiness(
            topicId: topicCustomerService,
            state: ReadinessState.notYet,
          ),
          MemberReadiness(
            topicId: topicStoreSafety,
            state: ReadinessState.ready,
          ),
          MemberReadiness(topicId: topicAml, state: ReadinessState.notStarted),
        ],
      ),
      TeamMember(
        userId: mohammadId,
        fullName: 'Mohammad A.',
        language: 'bn',
        readiness: [
          MemberReadiness(
            topicId: topicCustomerService,
            state: ReadinessState.notStarted,
          ),
          MemberReadiness(
            topicId: topicStoreSafety,
            state: ReadinessState.notYet,
          ),
          MemberReadiness(topicId: topicAml, state: ReadinessState.notStarted),
        ],
      ),
      TeamMember(
        userId: fatimahId,
        fullName: 'Fatimah Noor',
        lastActiveOn: '2026-10-30',
        nudgedToday: true,
        readiness: [
          MemberReadiness(
            topicId: topicCustomerService,
            state: ReadinessState.ready,
          ),
          MemberReadiness(
            topicId: topicStoreSafety,
            state: ReadinessState.notYet,
          ),
          MemberReadiness(topicId: topicAml, state: ReadinessState.ready),
        ],
      ),
    ],
  );

  static const privacy = PrivacyStatement(
    managerSees: [
      'Which topics you are ready on',
      'Team averages',
      'Whether you were active this week',
    ],
    hrSees: ['Hours on mandatory topics'],
    nobodySees: [
      'Which answer you chose',
      'What you saved',
      'When you watch',
      'Where you are',
    ],
  );
}
