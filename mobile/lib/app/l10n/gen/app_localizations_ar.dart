// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Arabic (`ar`).
class AppLocalizationsAr extends AppLocalizations {
  AppLocalizationsAr([String locale = 'ar']) : super(locale);

  @override
  String get appName => 'ريليرن';

  @override
  String get back => 'رجوع';

  @override
  String get close => 'إغلاق';

  @override
  String get retry => 'حاول مرة أخرى';

  @override
  String get cancel => 'إلغاء';

  @override
  String get save => 'حفظ';

  @override
  String get send => 'إرسال';

  @override
  String get continueLabel => 'متابعة';

  @override
  String get copyRequestId => 'نسخ رقم الطلب';

  @override
  String get requestIdCopied => 'تم نسخ رقم الطلب';

  @override
  String get errorGeneric => 'حدث خطأ ما. حاول مرة أخرى.';

  @override
  String get offlineBanner => 'لا يوجد اتصال. سنلحق بكل شيء حين تعود.';

  @override
  String get navToday => 'اليوم';

  @override
  String get navSaved => 'المحفوظات';

  @override
  String get navProgress => 'التقدم';

  @override
  String get navSettings => 'الإعدادات';

  @override
  String get navTeam => 'الفريق';

  @override
  String get signInTitle => 'تسجيل الدخول';

  @override
  String get companyCodeLabel => 'رمز الشركة';

  @override
  String get companyCodeHint => 'مثال: alwadi';

  @override
  String get companyLookup => 'ابحث عن شركتي';

  @override
  String get companyNotFound => 'لا توجد مساحة شركة بهذا الرمز. راجع مديرك.';

  @override
  String companyFound(String name) {
    return 'أهلًا بك في $name';
  }

  @override
  String get changeCompany => 'ليست شركتك؟';

  @override
  String get phoneLabel => 'رقم الجوال';

  @override
  String get phonePrefix => '+966';

  @override
  String get phoneHint => '5x xxx xxxx';

  @override
  String get sendCode => 'أرسل رمزي على واتساب';

  @override
  String get phoneNotOnList =>
      'هذا الرقم ليس في قائمة شركتك. اطلب من مديرك إضافتك.';

  @override
  String get otpTitle => 'أدخل الرمز الذي أرسلناه على واتساب';

  @override
  String otpSentTo(String phone) {
    return 'أُرسل إلى $phone';
  }

  @override
  String get otpHint => 'رمز من 4 أرقام';

  @override
  String otpWrong(String attempts) {
    return 'الرمز غير صحيح. بقيت $attempts محاولات.';
  }

  @override
  String get otpLocked => 'محاولات كثيرة. انتظر 15 دقيقة ثم اطلب رمزًا جديدًا.';

  @override
  String get otpExpired => 'انتهت صلاحية الرمز. اطلب رمزًا جديدًا.';

  @override
  String get otpTooMany =>
      'طلبت عدة رموز بالفعل. انتظر قليلًا قبل الطلب مجددًا.';

  @override
  String get otpResend => 'أرسل رمزًا جديدًا';

  @override
  String get changeNumber => 'تغيير الرقم';

  @override
  String get devCodeHint => 'نسخة التطوير: الرمز هو 1234';

  @override
  String welcomeTitle(String name) {
    return 'أهلًا $name';
  }

  @override
  String get welcomePromise1 => 'دقيقة واحدة في اليوم. لا أكثر أبدًا.';

  @override
  String get welcomePromise2 => 'سؤال بعد كل مقطع، حتى يثبت في الذاكرة.';

  @override
  String get welcomePromise3 =>
      'إجاباتك تبقى لك. المدير يرى الجاهزية لا الإجابات.';

  @override
  String get chooseSlot => 'متى تصلك دقيقتك؟';

  @override
  String get slotShiftStart => 'بداية الوردية';

  @override
  String get slotBreak => 'الاستراحة';

  @override
  String get slotShiftEnd => 'نهاية الوردية';

  @override
  String get startFirstMinute => 'ابدأ دقيقتي الأولى';

  @override
  String get todayTitle => 'اليوم';

  @override
  String greeting(String name) {
    return 'يومك سعيد، $name';
  }

  @override
  String newReelsCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count مقاطع جديدة',
      two: 'مقطعان جديدان',
      one: 'مقطع جديد واحد',
      zero: 'لا مقاطع جديدة',
    );
    return '$_temp0';
  }

  @override
  String reviewsCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count مراجعات',
      one: 'مراجعة واحدة',
      zero: 'بلا مراجعة',
    );
    return '$_temp0';
  }

  @override
  String aboutSeconds(int seconds) {
    return 'حوالي $seconds ثانية';
  }

  @override
  String streakChip(int days) {
    return 'سلسلة $days يومًا';
  }

  @override
  String yesterdayLine(int remembered, int total) {
    return 'بالأمس تذكرت $remembered من $total.';
  }

  @override
  String nextReviewOn(String date) {
    return 'المراجعة القادمة في $date.';
  }

  @override
  String get startMinute => 'ابدأ دقيقة اليوم';

  @override
  String get continueMinute => 'أكمل دقيقة اليوم';

  @override
  String get emptyTitle => 'لا شيء اليوم';

  @override
  String get emptyBody => 'استمتع بيومك. مقاطعك القادمة تصل مع ورديتك القادمة.';

  @override
  String get doneTitle => 'انتهى اليوم';

  @override
  String get doneBody => 'اكتملت دقيقتك. نراك في الوردية القادمة.';

  @override
  String get openSaved => 'المقاطع المحفوظة';

  @override
  String get todayLoadError => 'تعذّر تحميل خطة اليوم.';

  @override
  String get swipeHint => 'اسحب للأعلى للسؤال';

  @override
  String get swipeHintNext => 'اسحب للأعلى للمقطع التالي';

  @override
  String get answerFirstToast => 'أجب أولًا، ثم اسحب للأعلى';

  @override
  String get listenOnly => 'استماع فقط';

  @override
  String get captions => 'الترجمة';

  @override
  String get saveReel => 'حفظ';

  @override
  String get savedReel => 'محفوظ';

  @override
  String get whyThis => 'لماذا أرى هذا';

  @override
  String get remembered => 'تذكرت.';

  @override
  String get notQuite => 'ليس تمامًا.';

  @override
  String comesBackOn(String date) {
    return 'يعود في $date';
  }

  @override
  String reviewChip(int day) {
    return 'سُئل مجددًا · اليوم $day';
  }

  @override
  String get reviewLabel => 'مراجعة';

  @override
  String sourceLine(String citation) {
    return 'المصدر: $citation';
  }

  @override
  String presenterLine(String name) {
    return 'مع $name';
  }

  @override
  String get answerSavedOffline =>
      'حُفظت إجابتك. سترى النتيجة حين يعود الاتصال.';

  @override
  String doneRemembered(int remembered, int total) {
    return 'تذكرت $remembered من $total';
  }

  @override
  String streakLine(int days) {
    return 'سلسلة $days يومًا';
  }

  @override
  String freezeNote(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count تجميدات متبقية لأيام تفوّتها. الجمعة لا تُحسب أبدًا.',
      one: 'تجميد واحد متبقٍ ليوم تفوّته. الجمعة لا تُحسب أبدًا.',
      zero: 'لا تجميد متبقٍ هذا الأسبوع. الجمعة لا تُحسب أبدًا.',
    );
    return '$_temp0';
  }

  @override
  String get backToWork => 'العودة إلى العمل';

  @override
  String get nothingMore => 'هذا كل شيء لليوم.';

  @override
  String unitLabel(int index) {
    return 'الوحدة $index';
  }

  @override
  String get savedTitle => 'المحفوظات';

  @override
  String get searchHint => 'ابحث في المقاطع';

  @override
  String get allTopics => 'كل المواضيع';

  @override
  String get savedOnly => 'المحفوظ فقط';

  @override
  String get noResults => 'لا مقاطع مطابقة.';

  @override
  String get askForReel => 'اطلب مقطعًا';

  @override
  String get askForReelTitle => 'ما الذي يساعدك في عملك؟';

  @override
  String get askForReelHint => 'مثال: استرجاع بدون فاتورة';

  @override
  String get askForReelAnon => 'يرى مديرك الطلب، لا اسمك.';

  @override
  String get askSent => 'شكرًا. وصل طلبك.';

  @override
  String durationSeconds(int seconds) {
    return '$seconds ث';
  }

  @override
  String get progressTitle => 'التقدم';

  @override
  String rememberedHeadline(int pct) {
    return 'تتذكر $pct%';
  }

  @override
  String rememberedCount(int count) {
    return '$count أشياء تتذكرها';
  }

  @override
  String get weeklyTitle => 'أسبوعًا بأسبوع';

  @override
  String get byTopicTitle => 'حسب الموضوع';

  @override
  String get milestonesTitle => 'المحطات';

  @override
  String longestStreak(int days) {
    return 'أطول سلسلة: $days يومًا';
  }

  @override
  String get reached => 'تحققت';

  @override
  String get notYetReached => 'ليس بعد';

  @override
  String get settingsTitle => 'الإعدادات';

  @override
  String get language => 'اللغة';

  @override
  String get langAr => 'العربية';

  @override
  String get langEn => 'English';

  @override
  String get langUr => 'اردو';

  @override
  String get langBn => 'বাংলা';

  @override
  String get deliverySlotSetting => 'متى تصل دقيقتك';

  @override
  String get channelSetting => 'أين تصل';

  @override
  String get channelApp => 'التطبيق';

  @override
  String get channelWhatsapp => 'واتساب';

  @override
  String get channelBoth => 'كلاهما';

  @override
  String get captionsSetting => 'الترجمة مفعّلة افتراضيًا';

  @override
  String get remindersSetting => 'التذكيرات';

  @override
  String get streakFreezeSetting => 'تجميد السلسلة';

  @override
  String streakFreezeBody(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count متبقية هذا الأسبوع. تغطي أيامًا تفوّتها.',
      one: 'واحد متبقٍ هذا الأسبوع. يغطي يومًا تفوّته.',
      zero: 'لا شيء متبقٍ هذا الأسبوع.',
    );
    return '$_temp0';
  }

  @override
  String get extraReelsSetting => 'مقاطع إضافية';

  @override
  String get extraReelsBody =>
      'مقطع جديد إضافي في اليوم. ما زال أقل من دقيقتين.';

  @override
  String get pauseSetting => 'إيقاف لأسبوعين';

  @override
  String get pauseBody => 'تتوقف الإرسالات وسلسلتك حتى تعود.';

  @override
  String pausedUntil(String date) {
    return 'متوقف حتى $date';
  }

  @override
  String get resumeNow => 'استئناف الآن';

  @override
  String privacyTitle(String company) {
    return 'ما الذي تراه $company';
  }

  @override
  String get managerSees => 'مديرك يرى';

  @override
  String get hrSees => 'الموارد البشرية ترى';

  @override
  String get nobodySees => 'لا أحد يرى';

  @override
  String get downloadData => 'تنزيل كل ما هو مخزّن عني';

  @override
  String get exportQueued => 'سنرسل بياناتك على واتساب خلال يوم.';

  @override
  String get signOut => 'تسجيل الخروج';

  @override
  String get settingsSaved => 'تم الحفظ';

  @override
  String get teamTitle => 'الفريق';

  @override
  String teamRecall(int pct) {
    return 'تذكّر الفريق $pct%';
  }

  @override
  String activeThisWeek(int active, int size) {
    return '$active من $size نشطون هذا الأسبوع';
  }

  @override
  String get ready => 'جاهز';

  @override
  String get notYet => 'ليس بعد';

  @override
  String get notStarted => 'لم يبدأ';

  @override
  String get nudge => 'تذكير';

  @override
  String get nudgedToday => 'ذُكِّر اليوم';

  @override
  String nudgeSent(String name) {
    return 'أُرسل تذكير إلى $name';
  }

  @override
  String nudgeSkipped(String name) {
    return '$name ذُكِّر اليوم بالفعل';
  }

  @override
  String get recordReel => 'سجّل مقطعًا';

  @override
  String get privacyFootnote =>
      'ترى الجاهزية حسب الموضوع. لا إجابات ولا محفوظات ولا أوقات.';

  @override
  String get usesWhatsapp => 'واتساب';

  @override
  String get recordTitle => 'سجّل مقطعًا';

  @override
  String get recordTopic => 'الموضوع';

  @override
  String get recordPromptTitle => 'أسطرك الثلاثة';

  @override
  String get promptLine1 => 'السطر 1: الشيء الوحيد الذي يجب تذكره';

  @override
  String get promptLine2 => 'السطر 2: متى يهم';

  @override
  String get promptLine3 => 'السطر 3: كيف تفعله';

  @override
  String consentLabel(String company) {
    return 'أوافق على ظهور وجهي وصوتي في هذا المقطع لموظفي $company.';
  }

  @override
  String get startRecording => 'ابدأ التسجيل';

  @override
  String get stopRecording => 'إيقاف';

  @override
  String secondsLeft(int seconds) {
    return 'بقيت $seconds ث';
  }

  @override
  String get retake => 'إعادة';

  @override
  String get sendToReview => 'أرسل للمراجعة';

  @override
  String get sending => 'جارٍ الإرسال';

  @override
  String get sentToReview => 'أُرسل. سيراجعه فريق الاستوديو.';

  @override
  String get cameraUnavailable => 'الكاميرا غير متاحة على هذا الجهاز.';

  @override
  String get recordNeedsConsent => 'ضع علامة على مربع الموافقة للمتابعة.';

  @override
  String get recordNeedsPrompt => 'اكتب أسطرك الثلاثة أولًا.';

  @override
  String get recordingLabel => 'جارٍ التسجيل';

  @override
  String get keepGoing => 'أكمل';

  @override
  String get keepGoingHint => 'ريلات إضافية اخترناها لك، متى شئت.';

  @override
  String get sendReel => 'أرسل';

  @override
  String get notForMe => 'ليس لي';

  @override
  String get notForMeDone => 'تمام. ريلات أقل من هذا النوع.';

  @override
  String reasonShared(String name) {
    return 'أرسله $name';
  }

  @override
  String reasonMandatory(String topic) {
    return 'إلزامي · $topic';
  }

  @override
  String reasonFollowed(String topic) {
    return 'تتابع $topic';
  }

  @override
  String reasonInterest(String topic) {
    return 'لأنك تهتم بـ$topic';
  }

  @override
  String reasonPopular(String place) {
    return 'شائع في $place';
  }

  @override
  String reasonNew(String topic) {
    return 'جديد في $topic';
  }

  @override
  String get reasonExplore => 'شيء جديد لتجربته';

  @override
  String get reasonReview => 'مراجعة سريعة';

  @override
  String reasonFrom(String topic) {
    return 'من $topic';
  }

  @override
  String get practiceTitle => 'تدريب';

  @override
  String get practiceLoading => 'نختار ريلاتك التالية…';

  @override
  String get practiceError => 'تعذّر تحميل المزيد من الريلات.';

  @override
  String get breakTitle => 'وقت لاستراحة قصيرة';

  @override
  String get breakBody =>
      'تمدّد، وأبعد نظرك عن الشاشة، واشرب ماءً. ريلك التالي سينتظرك.';

  @override
  String get breakContinue => 'اسحب للأعلى عندما تكون جاهزًا';

  @override
  String get breakStop => 'توقف الآن';

  @override
  String get practiceEndTitle => 'هذا كل شيء الآن';

  @override
  String get practiceEndDailyCap => 'وصلت إلى وقت التدريب اليومي. نراك غدًا.';

  @override
  String get practiceEndSessionCap => 'هذه جلسة كاملة. عد لاحقًا للمزيد.';

  @override
  String get practiceEndOutsideHours => 'التدريب متاح خلال ساعات العمل.';

  @override
  String get practiceEndPaused => 'تذكيراتك وتدريبك متوقفة مؤقتًا.';

  @override
  String get practiceEndExhausted => 'شاهدت كل شيء حاليًا. ريلات جديدة قريبًا.';

  @override
  String get practiceEndDisabled => 'التدريب غير مفعّل في شركتك.';

  @override
  String get practiceBackToToday => 'العودة لليوم';

  @override
  String get shareTitle => 'أرسل إلى زملاء';

  @override
  String get shareSearch => 'ابحث بالاسم';

  @override
  String get shareNote => 'أضف ملاحظة (اختياري)';

  @override
  String get shareNone => 'لا يوجد زملاء.';

  @override
  String shareSent(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'أُرسل إلى $count زملاء',
      one: 'أُرسل إلى زميل واحد',
    );
    return '$_temp0';
  }

  @override
  String shareSendCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'أرسل إلى $count',
      zero: 'أرسل',
    );
    return '$_temp0';
  }

  @override
  String get topicsTitle => 'مواضيعك';

  @override
  String get topicsSubtitle =>
      'تابع ما تريد المزيد منه، واكتم ما لا يخصك. المواضيع الإلزامية تبقى دائمًا.';

  @override
  String get topicsPersonalized => 'خصّص ريلاتي';

  @override
  String get topicsPersonalizedHint =>
      'عند الإيقاف: تأتي الريلات بترتيب شركتك دون استخدام اهتماماتك.';

  @override
  String get topicFollow => 'تابع';

  @override
  String get topicMute => 'اكتم';

  @override
  String get topicRequired => 'إلزامي';

  @override
  String topicInterest(int value) {
    return 'الاهتمام $value %';
  }

  @override
  String get topicsTags => 'ما تميل إليه';

  @override
  String get topicsOpen => 'مواضيعك';

  @override
  String get topicsOpenHint => 'المتابعة والكتم وكيف تُختار ريلاتك';

  @override
  String get topicsMuteRequired => 'لا يمكن كتم المواضيع الإلزامية.';
}
