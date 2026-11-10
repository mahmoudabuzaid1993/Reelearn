// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get appName => 'Reelearn';

  @override
  String get back => 'Back';

  @override
  String get close => 'Close';

  @override
  String get retry => 'Try again';

  @override
  String get cancel => 'Cancel';

  @override
  String get save => 'Save';

  @override
  String get send => 'Send';

  @override
  String get continueLabel => 'Continue';

  @override
  String get copyRequestId => 'Copy request id';

  @override
  String get requestIdCopied => 'Request id copied';

  @override
  String get errorGeneric => 'Something went wrong. Try again.';

  @override
  String get offlineBanner =>
      'You\'re offline. We\'ll catch up as soon as you\'re back.';

  @override
  String get navToday => 'Today';

  @override
  String get navSaved => 'Saved';

  @override
  String get navProgress => 'Progress';

  @override
  String get navSettings => 'Settings';

  @override
  String get navTeam => 'Team';

  @override
  String get signInTitle => 'Sign in';

  @override
  String get companyCodeLabel => 'Company code';

  @override
  String get companyCodeHint => 'for example alwadi';

  @override
  String get companyLookup => 'Find my company';

  @override
  String get companyNotFound =>
      'No company space with that code. Check with your manager.';

  @override
  String companyFound(String name) {
    return 'Welcome to $name';
  }

  @override
  String get changeCompany => 'Not your company?';

  @override
  String get phoneLabel => 'Mobile number';

  @override
  String get phonePrefix => '+966';

  @override
  String get phoneHint => '5x xxx xxxx';

  @override
  String get sendCode => 'Send my code on WhatsApp';

  @override
  String get phoneNotOnList =>
      'This number isn\'t on your company\'s list. Ask your manager to add you.';

  @override
  String get otpTitle => 'Enter the code we sent on WhatsApp';

  @override
  String otpSentTo(String phone) {
    return 'Sent to $phone';
  }

  @override
  String get otpHint => '4-digit code';

  @override
  String otpWrong(String attempts) {
    return 'That code isn\'t right. $attempts attempts left.';
  }

  @override
  String get otpLocked =>
      'Too many tries. Wait 15 minutes, then ask for a new code.';

  @override
  String get otpExpired => 'That code has expired. Ask for a new one.';

  @override
  String get otpTooMany =>
      'You\'ve asked for a few codes already. Wait a little before asking again.';

  @override
  String get otpResend => 'Send a new code';

  @override
  String get changeNumber => 'Change number';

  @override
  String get devCodeHint => 'Development build: the code is 1234';

  @override
  String welcomeTitle(String name) {
    return 'Hi $name';
  }

  @override
  String get welcomePromise1 => 'One minute a day. Never more.';

  @override
  String get welcomePromise2 => 'A question after each reel, so it sticks.';

  @override
  String get welcomePromise3 =>
      'Your answers stay yours. Managers see readiness, not answers.';

  @override
  String get chooseSlot => 'When should your minute arrive?';

  @override
  String get slotShiftStart => 'Shift start';

  @override
  String get slotBreak => 'Break';

  @override
  String get slotShiftEnd => 'Shift end';

  @override
  String get startFirstMinute => 'Start my first minute';

  @override
  String get todayTitle => 'Today';

  @override
  String greeting(String name) {
    return 'Good day, $name';
  }

  @override
  String newReelsCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count new reels',
      one: '1 new reel',
      zero: 'no new reels',
    );
    return '$_temp0';
  }

  @override
  String reviewsCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count reviews',
      one: '1 review',
      zero: 'no review',
    );
    return '$_temp0';
  }

  @override
  String aboutSeconds(int seconds) {
    return 'about $seconds s';
  }

  @override
  String streakChip(int days) {
    return '$days-day streak';
  }

  @override
  String yesterdayLine(int remembered, int total) {
    return 'Yesterday you remembered $remembered of $total.';
  }

  @override
  String nextReviewOn(String date) {
    return 'Next review on $date.';
  }

  @override
  String get startMinute => 'Start today\'s minute';

  @override
  String get continueMinute => 'Continue today\'s minute';

  @override
  String get emptyTitle => 'Nothing for today';

  @override
  String get emptyBody =>
      'Enjoy your day. Your next reels arrive with your next shift.';

  @override
  String get doneTitle => 'Done for today';

  @override
  String get doneBody => 'Your minute is complete. See you at the next shift.';

  @override
  String get openSaved => 'Saved reels';

  @override
  String get todayLoadError => 'Couldn\'t load today\'s plan.';

  @override
  String get swipeHint => 'Swipe up for the question';

  @override
  String get swipeHintNext => 'Swipe up for the next reel';

  @override
  String get answerFirstToast => 'Answer first, then swipe up';

  @override
  String get listenOnly => 'Listen only';

  @override
  String get captions => 'Captions';

  @override
  String get saveReel => 'Save';

  @override
  String get savedReel => 'Saved';

  @override
  String get whyThis => 'Why am I seeing this';

  @override
  String get remembered => 'Remembered.';

  @override
  String get notQuite => 'Not quite.';

  @override
  String comesBackOn(String date) {
    return 'Comes back on $date';
  }

  @override
  String reviewChip(int day) {
    return 'Asked again · day $day';
  }

  @override
  String get reviewLabel => 'Review';

  @override
  String sourceLine(String citation) {
    return 'Source: $citation';
  }

  @override
  String presenterLine(String name) {
    return 'with $name';
  }

  @override
  String get answerSavedOffline =>
      'Answer saved. You\'ll see the result when you\'re back online.';

  @override
  String doneRemembered(int remembered, int total) {
    return 'Remembered $remembered of $total';
  }

  @override
  String streakLine(int days) {
    return '$days-day streak';
  }

  @override
  String freezeNote(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          '$count streak freezes left for days you miss. Fridays never count.',
      one: '1 streak freeze left for a day you miss. Fridays never count.',
      zero: 'No streak freeze left this week. Fridays never count.',
    );
    return '$_temp0';
  }

  @override
  String get backToWork => 'Back to work';

  @override
  String get nothingMore => 'That\'s all for today.';

  @override
  String unitLabel(int index) {
    return 'Unit $index';
  }

  @override
  String get savedTitle => 'Saved';

  @override
  String get searchHint => 'Search reels';

  @override
  String get allTopics => 'All topics';

  @override
  String get savedOnly => 'Saved only';

  @override
  String get noResults => 'No reels match.';

  @override
  String get askForReel => 'Ask for a reel';

  @override
  String get askForReelTitle => 'What would help you at work?';

  @override
  String get askForReelHint => 'for example: a refund without a receipt';

  @override
  String get askForReelAnon => 'Your manager sees the request, not your name.';

  @override
  String get askSent => 'Thanks. Your request is in.';

  @override
  String durationSeconds(int seconds) {
    return '$seconds s';
  }

  @override
  String get progressTitle => 'Progress';

  @override
  String rememberedHeadline(int pct) {
    return 'You remember $pct%';
  }

  @override
  String rememberedCount(int count) {
    return '$count things remembered';
  }

  @override
  String get weeklyTitle => 'Week by week';

  @override
  String get byTopicTitle => 'By topic';

  @override
  String get milestonesTitle => 'Milestones';

  @override
  String longestStreak(int days) {
    return 'Longest streak: $days days';
  }

  @override
  String get reached => 'Reached';

  @override
  String get notYetReached => 'Not yet';

  @override
  String get settingsTitle => 'Settings';

  @override
  String get language => 'Language';

  @override
  String get langAr => 'العربية';

  @override
  String get langEn => 'English';

  @override
  String get langUr => 'اردو';

  @override
  String get langBn => 'বাংলা';

  @override
  String get deliverySlotSetting => 'When your minute arrives';

  @override
  String get channelSetting => 'Where it arrives';

  @override
  String get channelApp => 'App';

  @override
  String get channelWhatsapp => 'WhatsApp';

  @override
  String get channelBoth => 'Both';

  @override
  String get captionsSetting => 'Captions on by default';

  @override
  String get remindersSetting => 'Reminders';

  @override
  String get streakFreezeSetting => 'Streak freeze';

  @override
  String streakFreezeBody(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count left this week. They cover days you miss.',
      one: '1 left this week. It covers a day you miss.',
      zero: 'None left this week.',
    );
    return '$_temp0';
  }

  @override
  String get extraReelsSetting => 'Extra reels';

  @override
  String get extraReelsBody =>
      'One more new reel a day. Still under two minutes.';

  @override
  String get pauseSetting => 'Pause for two weeks';

  @override
  String get pauseBody => 'Deliveries and your streak stop until you\'re back.';

  @override
  String pausedUntil(String date) {
    return 'Paused until $date';
  }

  @override
  String get resumeNow => 'Resume now';

  @override
  String privacyTitle(String company) {
    return 'What $company can see';
  }

  @override
  String get managerSees => 'Your manager sees';

  @override
  String get hrSees => 'HR sees';

  @override
  String get nobodySees => 'Nobody sees';

  @override
  String get downloadData => 'Download everything stored about me';

  @override
  String get exportQueued => 'We\'ll send your data on WhatsApp within a day.';

  @override
  String get signOut => 'Sign out';

  @override
  String get settingsSaved => 'Saved';

  @override
  String get teamTitle => 'Team';

  @override
  String teamRecall(int pct) {
    return 'Team recall $pct%';
  }

  @override
  String activeThisWeek(int active, int size) {
    return '$active of $size active this week';
  }

  @override
  String get ready => 'Ready';

  @override
  String get notYet => 'Not yet';

  @override
  String get notStarted => 'Not started';

  @override
  String get nudge => 'Nudge';

  @override
  String get nudgedToday => 'Nudged today';

  @override
  String nudgeSent(String name) {
    return 'Reminder sent to $name';
  }

  @override
  String nudgeSkipped(String name) {
    return '$name was already nudged today';
  }

  @override
  String get recordReel => 'Record a reel';

  @override
  String get privacyFootnote =>
      'You see readiness per topic. Never answers, saves, or times.';

  @override
  String get usesWhatsapp => 'WhatsApp';

  @override
  String get recordTitle => 'Record a reel';

  @override
  String get recordTopic => 'Topic';

  @override
  String get recordPromptTitle => 'Your three lines';

  @override
  String get promptLine1 => 'Line 1: the one thing to remember';

  @override
  String get promptLine2 => 'Line 2: when it matters';

  @override
  String get promptLine3 => 'Line 3: how to do it';

  @override
  String consentLabel(String company) {
    return 'I agree that my face and voice appear in this reel for $company staff.';
  }

  @override
  String get startRecording => 'Start recording';

  @override
  String get stopRecording => 'Stop';

  @override
  String secondsLeft(int seconds) {
    return '$seconds s left';
  }

  @override
  String get retake => 'Retake';

  @override
  String get sendToReview => 'Send to review';

  @override
  String get sending => 'Sending';

  @override
  String get sentToReview => 'Sent. The studio team will review it.';

  @override
  String get cameraUnavailable => 'The camera isn\'t available on this device.';

  @override
  String get recordNeedsConsent => 'Tick the consent box to continue.';

  @override
  String get recordNeedsPrompt => 'Write your three lines first.';

  @override
  String get recordingLabel => 'Recording';

  @override
  String get keepGoing => 'Keep going';

  @override
  String get keepGoingHint =>
      'A few more reels picked for you, whenever you like.';

  @override
  String get sendReel => 'Send';

  @override
  String get notForMe => 'Not for me';

  @override
  String get notForMeDone => 'Got it. Fewer reels like this.';

  @override
  String reasonShared(String name) {
    return 'Sent by $name';
  }

  @override
  String reasonMandatory(String topic) {
    return 'Required · $topic';
  }

  @override
  String reasonFollowed(String topic) {
    return 'You follow $topic';
  }

  @override
  String reasonInterest(String topic) {
    return 'Because you like $topic';
  }

  @override
  String reasonPopular(String place) {
    return 'Popular in $place';
  }

  @override
  String reasonNew(String topic) {
    return 'New in $topic';
  }

  @override
  String get reasonExplore => 'Something new to try';

  @override
  String get reasonReview => 'Quick review';

  @override
  String reasonFrom(String topic) {
    return 'From $topic';
  }

  @override
  String get practiceTitle => 'Practice';

  @override
  String get practiceLoading => 'Picking your next reels…';

  @override
  String get practiceError => 'Couldn\'t load more reels.';

  @override
  String get breakTitle => 'Time for a short pause';

  @override
  String get breakBody =>
      'Stretch, look away from the screen, have some water. Your next reel will wait.';

  @override
  String get breakContinue => 'Swipe up when you\'re ready';

  @override
  String get breakStop => 'Stop for now';

  @override
  String get practiceEndTitle => 'That\'s it for now';

  @override
  String get practiceEndDailyCap =>
      'You\'ve reached today\'s practice time. See you tomorrow.';

  @override
  String get practiceEndSessionCap =>
      'That\'s a full session. Come back later for more.';

  @override
  String get practiceEndOutsideHours => 'Practice opens during work hours.';

  @override
  String get practiceEndPaused => 'Your reminders and practice are paused.';

  @override
  String get practiceEndExhausted =>
      'You\'ve seen everything for now. New reels arrive soon.';

  @override
  String get practiceEndDisabled => 'Your company has practice switched off.';

  @override
  String get practiceBackToToday => 'Back to today';

  @override
  String get shareTitle => 'Send to colleagues';

  @override
  String get shareSearch => 'Search by name';

  @override
  String get shareNote => 'Add a note (optional)';

  @override
  String get shareNone => 'No colleagues found.';

  @override
  String shareSent(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Sent to $count colleagues',
      one: 'Sent to 1 colleague',
    );
    return '$_temp0';
  }

  @override
  String shareSendCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Send to $count',
      zero: 'Send',
    );
    return '$_temp0';
  }

  @override
  String get topicsTitle => 'Your topics';

  @override
  String get topicsSubtitle =>
      'Follow what you want more of, mute what isn\'t for you. Required topics always stay.';

  @override
  String get topicsPersonalized => 'Personalise my reels';

  @override
  String get topicsPersonalizedHint =>
      'Off: reels come in the order your company set, without using your interests.';

  @override
  String get topicFollow => 'Follow';

  @override
  String get topicMute => 'Mute';

  @override
  String get topicRequired => 'Required';

  @override
  String topicInterest(int value) {
    return 'Interest $value %';
  }

  @override
  String get topicsTags => 'What you lean toward';

  @override
  String get topicsOpen => 'Your topics';

  @override
  String get topicsOpenHint => 'Follow, mute, and how reels are chosen for you';

  @override
  String get topicsMuteRequired => 'Required topics can\'t be muted.';
}
