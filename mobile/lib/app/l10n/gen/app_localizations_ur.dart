// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Urdu (`ur`).
class AppLocalizationsUr extends AppLocalizations {
  AppLocalizationsUr([String locale = 'ur']) : super(locale);

  @override
  String get appName => 'ریلرن';

  @override
  String get back => 'واپس';

  @override
  String get close => 'بند کریں';

  @override
  String get retry => 'دوبارہ کوشش کریں';

  @override
  String get cancel => 'منسوخ';

  @override
  String get save => 'محفوظ کریں';

  @override
  String get send => 'بھیجیں';

  @override
  String get continueLabel => 'جاری رکھیں';

  @override
  String get copyRequestId => 'درخواست آئی ڈی کاپی کریں';

  @override
  String get requestIdCopied => 'درخواست آئی ڈی کاپی ہو گئی';

  @override
  String get errorGeneric => 'کچھ غلط ہو گیا۔ دوبارہ کوشش کریں۔';

  @override
  String get offlineBanner =>
      'آپ آف لائن ہیں۔ واپس آتے ہی ہم سب کچھ اپ ڈیٹ کر دیں گے۔';

  @override
  String get navToday => 'آج';

  @override
  String get navSaved => 'محفوظ';

  @override
  String get navProgress => 'پیش رفت';

  @override
  String get navSettings => 'ترتیبات';

  @override
  String get navTeam => 'ٹیم';

  @override
  String get signInTitle => 'سائن ان';

  @override
  String get companyCodeLabel => 'کمپنی کوڈ';

  @override
  String get companyCodeHint => 'مثلاً alwadi';

  @override
  String get companyLookup => 'میری کمپنی تلاش کریں';

  @override
  String get companyNotFound =>
      'اس کوڈ سے کوئی کمپنی نہیں ملی۔ اپنے مینیجر سے پوچھیں۔';

  @override
  String companyFound(String name) {
    return '$name میں خوش آمدید';
  }

  @override
  String get changeCompany => 'آپ کی کمپنی نہیں؟';

  @override
  String get phoneLabel => 'موبائل نمبر';

  @override
  String get phonePrefix => '+966';

  @override
  String get phoneHint => '5x xxx xxxx';

  @override
  String get sendCode => 'میرا کوڈ واٹس ایپ پر بھیجیں';

  @override
  String get phoneNotOnList =>
      'یہ نمبر آپ کی کمپنی کی فہرست میں نہیں ہے۔ اپنے مینیجر سے کہیں کہ آپ کو شامل کریں۔';

  @override
  String get otpTitle => 'واٹس ایپ پر بھیجا گیا کوڈ درج کریں';

  @override
  String otpSentTo(String phone) {
    return '$phone پر بھیجا گیا';
  }

  @override
  String get otpHint => '4 ہندسوں کا کوڈ';

  @override
  String otpWrong(String attempts) {
    return 'یہ کوڈ درست نہیں۔ $attempts کوششیں باقی ہیں۔';
  }

  @override
  String get otpLocked =>
      'بہت زیادہ کوششیں۔ 15 منٹ انتظار کریں، پھر نیا کوڈ مانگیں۔';

  @override
  String get otpExpired => 'اس کوڈ کی میعاد ختم ہو گئی۔ نیا کوڈ مانگیں۔';

  @override
  String get otpTooMany =>
      'آپ پہلے ہی کئی کوڈ مانگ چکے ہیں۔ تھوڑا انتظار کریں۔';

  @override
  String get otpResend => 'نیا کوڈ بھیجیں';

  @override
  String get changeNumber => 'نمبر بدلیں';

  @override
  String get devCodeHint => 'ڈیولپمنٹ بلڈ: کوڈ 1234 ہے';

  @override
  String welcomeTitle(String name) {
    return 'ہیلو $name';
  }

  @override
  String get welcomePromise1 => 'روزانہ ایک منٹ۔ اس سے زیادہ کبھی نہیں۔';

  @override
  String get welcomePromise2 => 'ہر ریل کے بعد ایک سوال، تاکہ یاد رہے۔';

  @override
  String get welcomePromise3 =>
      'آپ کے جوابات آپ کے ہیں۔ مینیجر تیاری دیکھتے ہیں، جوابات نہیں۔';

  @override
  String get chooseSlot => 'آپ کا منٹ کب آئے؟';

  @override
  String get slotShiftStart => 'شفٹ کا آغاز';

  @override
  String get slotBreak => 'وقفہ';

  @override
  String get slotShiftEnd => 'شفٹ کا اختتام';

  @override
  String get startFirstMinute => 'میرا پہلا منٹ شروع کریں';

  @override
  String get todayTitle => 'آج';

  @override
  String greeting(String name) {
    return 'خوش آمدید، $name';
  }

  @override
  String newReelsCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count نئی ریلیں',
      one: '1 نئی ریل',
      zero: 'کوئی نئی ریل نہیں',
    );
    return '$_temp0';
  }

  @override
  String reviewsCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count جائزے',
      one: '1 جائزہ',
      zero: 'کوئی جائزہ نہیں',
    );
    return '$_temp0';
  }

  @override
  String aboutSeconds(int seconds) {
    return 'تقریباً $seconds سیکنڈ';
  }

  @override
  String streakChip(int days) {
    return '$days دن کا سلسلہ';
  }

  @override
  String yesterdayLine(int remembered, int total) {
    return 'کل آپ کو $total میں سے $remembered یاد رہے۔';
  }

  @override
  String nextReviewOn(String date) {
    return 'اگلا جائزہ $date کو۔';
  }

  @override
  String get startMinute => 'آج کا منٹ شروع کریں';

  @override
  String get continueMinute => 'آج کا منٹ جاری رکھیں';

  @override
  String get emptyTitle => 'آج کچھ نہیں';

  @override
  String get emptyBody =>
      'اپنا دن انجوائے کریں۔ اگلی ریلیں اگلی شفٹ کے ساتھ آئیں گی۔';

  @override
  String get doneTitle => 'آج کا کام مکمل';

  @override
  String get doneBody => 'آپ کا منٹ مکمل ہو گیا۔ اگلی شفٹ پر ملتے ہیں۔';

  @override
  String get openSaved => 'محفوظ ریلیں';

  @override
  String get todayLoadError => 'آج کا پلان لوڈ نہیں ہو سکا۔';

  @override
  String get swipeHint => 'سوال کے لیے اوپر سوائپ کریں';

  @override
  String get swipeHintNext => 'اگلی ریل کے لیے اوپر سوائپ کریں';

  @override
  String get answerFirstToast => 'پہلے جواب دیں، پھر اوپر سوائپ کریں';

  @override
  String get listenOnly => 'صرف سنیں';

  @override
  String get captions => 'کیپشن';

  @override
  String get saveReel => 'محفوظ کریں';

  @override
  String get savedReel => 'محفوظ';

  @override
  String get whyThis => 'مجھے یہ کیوں دکھ رہا ہے';

  @override
  String get remembered => 'یاد رہا۔';

  @override
  String get notQuite => 'بالکل نہیں۔';

  @override
  String comesBackOn(String date) {
    return '$date کو واپس آئے گا';
  }

  @override
  String reviewChip(int day) {
    return 'دوبارہ پوچھا گیا · دن $day';
  }

  @override
  String get reviewLabel => 'جائزہ';

  @override
  String sourceLine(String citation) {
    return 'ماخذ: $citation';
  }

  @override
  String presenterLine(String name) {
    return '$name کے ساتھ';
  }

  @override
  String get answerSavedOffline =>
      'جواب محفوظ ہو گیا۔ آن لائن آنے پر نتیجہ دیکھیں گے۔';

  @override
  String doneRemembered(int remembered, int total) {
    return '$total میں سے $remembered یاد رہے';
  }

  @override
  String streakLine(int days) {
    return '$days دن کا سلسلہ';
  }

  @override
  String freezeNote(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'چھوٹے ہوئے دنوں کے لیے $count فریز باقی ہیں۔ جمعہ کبھی شمار نہیں ہوتا۔',
      one: 'چھوٹے ہوئے دن کے لیے 1 فریز باقی ہے۔ جمعہ کبھی شمار نہیں ہوتا۔',
      zero: 'اس ہفتے کوئی فریز باقی نہیں۔ جمعہ کبھی شمار نہیں ہوتا۔',
    );
    return '$_temp0';
  }

  @override
  String get backToWork => 'کام پر واپس';

  @override
  String get nothingMore => 'آج کے لیے بس اتنا ہی۔';

  @override
  String unitLabel(int index) {
    return 'یونٹ $index';
  }

  @override
  String get savedTitle => 'محفوظ';

  @override
  String get searchHint => 'ریلیں تلاش کریں';

  @override
  String get allTopics => 'تمام موضوعات';

  @override
  String get savedOnly => 'صرف محفوظ';

  @override
  String get noResults => 'کوئی ریل نہیں ملی۔';

  @override
  String get askForReel => 'ریل کی درخواست کریں';

  @override
  String get askForReelTitle => 'کام میں کیا چیز مدد دے گی؟';

  @override
  String get askForReelHint => 'مثلاً: رسید کے بغیر واپسی';

  @override
  String get askForReelAnon =>
      'آپ کا مینیجر درخواست دیکھتا ہے، آپ کا نام نہیں۔';

  @override
  String get askSent => 'شکریہ۔ آپ کی درخواست پہنچ گئی۔';

  @override
  String durationSeconds(int seconds) {
    return '$seconds س';
  }

  @override
  String get progressTitle => 'پیش رفت';

  @override
  String rememberedHeadline(int pct) {
    return 'آپ کو $pct% یاد ہے';
  }

  @override
  String rememberedCount(int count) {
    return '$count چیزیں یاد ہیں';
  }

  @override
  String get weeklyTitle => 'ہفتہ بہ ہفتہ';

  @override
  String get byTopicTitle => 'موضوع کے لحاظ سے';

  @override
  String get milestonesTitle => 'سنگ میل';

  @override
  String longestStreak(int days) {
    return 'طویل ترین سلسلہ: $days دن';
  }

  @override
  String get reached => 'حاصل';

  @override
  String get notYetReached => 'ابھی نہیں';

  @override
  String get settingsTitle => 'ترتیبات';

  @override
  String get language => 'زبان';

  @override
  String get langAr => 'العربية';

  @override
  String get langEn => 'English';

  @override
  String get langUr => 'اردو';

  @override
  String get langBn => 'বাংলা';

  @override
  String get deliverySlotSetting => 'آپ کا منٹ کب آئے';

  @override
  String get channelSetting => 'کہاں آئے';

  @override
  String get channelApp => 'ایپ';

  @override
  String get channelWhatsapp => 'واٹس ایپ';

  @override
  String get channelBoth => 'دونوں';

  @override
  String get captionsSetting => 'کیپشن ڈیفالٹ طور پر آن';

  @override
  String get remindersSetting => 'یاد دہانیاں';

  @override
  String get streakFreezeSetting => 'سلسلہ فریز';

  @override
  String streakFreezeBody(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'اس ہفتے $count باقی ہیں۔ یہ چھوٹے ہوئے دن پورے کرتے ہیں۔',
      one: 'اس ہفتے 1 باقی ہے۔ یہ چھوٹا ہوا دن پورا کرتا ہے۔',
      zero: 'اس ہفتے کوئی باقی نہیں۔',
    );
    return '$_temp0';
  }

  @override
  String get extraReelsSetting => 'اضافی ریلیں';

  @override
  String get extraReelsBody => 'روزانہ ایک اور نئی ریل۔ پھر بھی دو منٹ سے کم۔';

  @override
  String get pauseSetting => 'دو ہفتے کے لیے روکیں';

  @override
  String get pauseBody => 'آپ کی واپسی تک ترسیل اور سلسلہ رک جاتا ہے۔';

  @override
  String pausedUntil(String date) {
    return '$date تک رکا ہوا';
  }

  @override
  String get resumeNow => 'ابھی دوبارہ شروع کریں';

  @override
  String privacyTitle(String company) {
    return '$company کیا دیکھ سکتی ہے';
  }

  @override
  String get managerSees => 'آپ کا مینیجر دیکھتا ہے';

  @override
  String get hrSees => 'ایچ آر دیکھتا ہے';

  @override
  String get nobodySees => 'کوئی نہیں دیکھتا';

  @override
  String get downloadData => 'میرے بارے میں محفوظ سب کچھ ڈاؤن لوڈ کریں';

  @override
  String get exportQueued =>
      'ہم ایک دن کے اندر آپ کا ڈیٹا واٹس ایپ پر بھیج دیں گے۔';

  @override
  String get signOut => 'سائن آؤٹ';

  @override
  String get settingsSaved => 'محفوظ ہو گیا';

  @override
  String get teamTitle => 'ٹیم';

  @override
  String teamRecall(int pct) {
    return 'ٹیم کی یادداشت $pct%';
  }

  @override
  String activeThisWeek(int active, int size) {
    return '$size میں سے $active اس ہفتے فعال';
  }

  @override
  String get ready => 'تیار';

  @override
  String get notYet => 'ابھی نہیں';

  @override
  String get notStarted => 'شروع نہیں ہوا';

  @override
  String get nudge => 'یاد دہانی';

  @override
  String get nudgedToday => 'آج یاد دہانی ہو چکی';

  @override
  String nudgeSent(String name) {
    return '$name کو یاد دہانی بھیج دی گئی';
  }

  @override
  String nudgeSkipped(String name) {
    return '$name کو آج پہلے ہی یاد دہانی ہو چکی';
  }

  @override
  String get recordReel => 'ریل ریکارڈ کریں';

  @override
  String get privacyFootnote =>
      'آپ موضوع کے لحاظ سے تیاری دیکھتے ہیں۔ جوابات، محفوظات یا اوقات کبھی نہیں۔';

  @override
  String get usesWhatsapp => 'واٹس ایپ';

  @override
  String get recordTitle => 'ریل ریکارڈ کریں';

  @override
  String get recordTopic => 'موضوع';

  @override
  String get recordPromptTitle => 'آپ کی تین سطریں';

  @override
  String get promptLine1 => 'سطر 1: یاد رکھنے والی ایک بات';

  @override
  String get promptLine2 => 'سطر 2: یہ کب اہم ہے';

  @override
  String get promptLine3 => 'سطر 3: اسے کیسے کریں';

  @override
  String consentLabel(String company) {
    return 'میں متفق ہوں کہ میرا چہرہ اور آواز $company کے عملے کے لیے اس ریل میں دکھائی دیں۔';
  }

  @override
  String get startRecording => 'ریکارڈنگ شروع کریں';

  @override
  String get stopRecording => 'روکیں';

  @override
  String secondsLeft(int seconds) {
    return '$seconds س باقی';
  }

  @override
  String get retake => 'دوبارہ لیں';

  @override
  String get sendToReview => 'جائزے کے لیے بھیجیں';

  @override
  String get sending => 'بھیجا جا رہا ہے';

  @override
  String get sentToReview => 'بھیج دیا گیا۔ اسٹوڈیو ٹیم اس کا جائزہ لے گی۔';

  @override
  String get cameraUnavailable => 'اس ڈیوائس پر کیمرا دستیاب نہیں۔';

  @override
  String get recordNeedsConsent => 'جاری رکھنے کے لیے رضامندی کا خانہ ٹک کریں۔';

  @override
  String get recordNeedsPrompt => 'پہلے اپنی تین سطریں لکھیں۔';

  @override
  String get recordingLabel => 'ریکارڈنگ';

  @override
  String get keepGoing => 'جاری رکھیں';

  @override
  String get keepGoingHint => 'آپ کے لیے چنی گئی چند اور ریلز، جب چاہیں۔';

  @override
  String get sendReel => 'بھیجیں';

  @override
  String get notForMe => 'میرے لیے نہیں';

  @override
  String get notForMeDone => 'ٹھیک ہے۔ ایسی ریلز کم ہوں گی۔';

  @override
  String reasonShared(String name) {
    return '$name نے بھیجی';
  }

  @override
  String reasonMandatory(String topic) {
    return 'لازمی · $topic';
  }

  @override
  String reasonFollowed(String topic) {
    return 'آپ $topic فالو کرتے ہیں';
  }

  @override
  String reasonInterest(String topic) {
    return 'کیونکہ آپ کو $topic پسند ہے';
  }

  @override
  String reasonPopular(String place) {
    return '$place میں مقبول';
  }

  @override
  String reasonNew(String topic) {
    return '$topic میں نیا';
  }

  @override
  String get reasonExplore => 'آزمانے کے لیے کچھ نیا';

  @override
  String get reasonReview => 'فوری دہرائی';

  @override
  String reasonFrom(String topic) {
    return '$topic سے';
  }

  @override
  String get practiceTitle => 'مشق';

  @override
  String get practiceLoading => 'آپ کی اگلی ریلز چن رہے ہیں…';

  @override
  String get practiceError => 'مزید ریلز لوڈ نہیں ہو سکیں۔';

  @override
  String get breakTitle => 'مختصر وقفے کا وقت';

  @override
  String get breakBody =>
      'انگڑائی لیں، اسکرین سے نظر ہٹائیں، پانی پئیں۔ اگلی ریل انتظار کرے گی۔';

  @override
  String get breakContinue => 'تیار ہوں تو اوپر سوائپ کریں';

  @override
  String get breakStop => 'ابھی رک جائیں';

  @override
  String get practiceEndTitle => 'فی الحال اتنا ہی';

  @override
  String get practiceEndDailyCap =>
      'آج کا مشق کا وقت پورا ہو گیا۔ کل ملتے ہیں۔';

  @override
  String get practiceEndSessionCap =>
      'یہ ایک مکمل سیشن تھا۔ مزید کے لیے بعد میں آئیں۔';

  @override
  String get practiceEndOutsideHours => 'مشق کام کے اوقات میں کھلتی ہے۔';

  @override
  String get practiceEndPaused => 'آپ کی یاد دہانیاں اور مشق رکی ہوئی ہیں۔';

  @override
  String get practiceEndExhausted =>
      'آپ نے فی الحال سب دیکھ لیا۔ نئی ریلز جلد آئیں گی۔';

  @override
  String get practiceEndDisabled => 'آپ کی کمپنی میں مشق بند ہے۔';

  @override
  String get practiceBackToToday => 'آج پر واپس';

  @override
  String get shareTitle => 'ساتھیوں کو بھیجیں';

  @override
  String get shareSearch => 'نام سے تلاش کریں';

  @override
  String get shareNote => 'نوٹ شامل کریں (اختیاری)';

  @override
  String get shareNone => 'کوئی ساتھی نہیں ملا۔';

  @override
  String shareSent(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count ساتھیوں کو بھیجی گئی',
      one: '1 ساتھی کو بھیجی گئی',
    );
    return '$_temp0';
  }

  @override
  String shareSendCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count کو بھیجیں',
      zero: 'بھیجیں',
    );
    return '$_temp0';
  }

  @override
  String get topicsTitle => 'آپ کے موضوعات';

  @override
  String get topicsSubtitle =>
      'جو زیادہ چاہیں اسے فالو کریں، جو آپ کے لیے نہیں اسے خاموش کریں۔ لازمی موضوعات ہمیشہ رہتے ہیں۔';

  @override
  String get topicsPersonalized => 'میری ریلز ذاتی بنائیں';

  @override
  String get topicsPersonalizedHint =>
      'بند: ریلز آپ کی کمپنی کی ترتیب میں آتی ہیں، آپ کی دلچسپی کے بغیر۔';

  @override
  String get topicFollow => 'فالو';

  @override
  String get topicMute => 'خاموش';

  @override
  String get topicRequired => 'لازمی';

  @override
  String topicInterest(int value) {
    return 'دلچسپی $value %';
  }

  @override
  String get topicsTags => 'آپ کا رجحان';

  @override
  String get topicsOpen => 'آپ کے موضوعات';

  @override
  String get topicsOpenHint => 'فالو، خاموش، اور آپ کی ریلز کیسے چنی جاتی ہیں';

  @override
  String get topicsMuteRequired => 'لازمی موضوعات خاموش نہیں ہو سکتے۔';
}
