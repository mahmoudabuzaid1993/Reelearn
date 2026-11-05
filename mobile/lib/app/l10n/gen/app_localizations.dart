import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_ar.dart';
import 'app_localizations_en.dart';
import 'app_localizations_ur.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'gen/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
      : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
    delegate,
    GlobalMaterialLocalizations.delegate,
    GlobalCupertinoLocalizations.delegate,
    GlobalWidgetsLocalizations.delegate,
  ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('ar'),
    Locale('en'),
    Locale('ur')
  ];

  /// No description provided for @appName.
  ///
  /// In en, this message translates to:
  /// **'Reelearn'**
  String get appName;

  /// No description provided for @back.
  ///
  /// In en, this message translates to:
  /// **'Back'**
  String get back;

  /// No description provided for @close.
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get close;

  /// No description provided for @retry.
  ///
  /// In en, this message translates to:
  /// **'Try again'**
  String get retry;

  /// No description provided for @cancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get cancel;

  /// No description provided for @save.
  ///
  /// In en, this message translates to:
  /// **'Save'**
  String get save;

  /// No description provided for @send.
  ///
  /// In en, this message translates to:
  /// **'Send'**
  String get send;

  /// No description provided for @continueLabel.
  ///
  /// In en, this message translates to:
  /// **'Continue'**
  String get continueLabel;

  /// No description provided for @copyRequestId.
  ///
  /// In en, this message translates to:
  /// **'Copy request id'**
  String get copyRequestId;

  /// No description provided for @requestIdCopied.
  ///
  /// In en, this message translates to:
  /// **'Request id copied'**
  String get requestIdCopied;

  /// No description provided for @errorGeneric.
  ///
  /// In en, this message translates to:
  /// **'Something went wrong. Try again.'**
  String get errorGeneric;

  /// No description provided for @offlineBanner.
  ///
  /// In en, this message translates to:
  /// **'You\'re offline. We\'ll catch up as soon as you\'re back.'**
  String get offlineBanner;

  /// No description provided for @navToday.
  ///
  /// In en, this message translates to:
  /// **'Today'**
  String get navToday;

  /// No description provided for @navSaved.
  ///
  /// In en, this message translates to:
  /// **'Saved'**
  String get navSaved;

  /// No description provided for @navProgress.
  ///
  /// In en, this message translates to:
  /// **'Progress'**
  String get navProgress;

  /// No description provided for @navSettings.
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get navSettings;

  /// No description provided for @navTeam.
  ///
  /// In en, this message translates to:
  /// **'Team'**
  String get navTeam;

  /// No description provided for @signInTitle.
  ///
  /// In en, this message translates to:
  /// **'Sign in'**
  String get signInTitle;

  /// No description provided for @companyCodeLabel.
  ///
  /// In en, this message translates to:
  /// **'Company code'**
  String get companyCodeLabel;

  /// No description provided for @companyCodeHint.
  ///
  /// In en, this message translates to:
  /// **'for example alwadi'**
  String get companyCodeHint;

  /// No description provided for @companyLookup.
  ///
  /// In en, this message translates to:
  /// **'Find my company'**
  String get companyLookup;

  /// No description provided for @companyNotFound.
  ///
  /// In en, this message translates to:
  /// **'No company space with that code. Check with your manager.'**
  String get companyNotFound;

  /// No description provided for @companyFound.
  ///
  /// In en, this message translates to:
  /// **'Welcome to {name}'**
  String companyFound(String name);

  /// No description provided for @changeCompany.
  ///
  /// In en, this message translates to:
  /// **'Not your company?'**
  String get changeCompany;

  /// No description provided for @phoneLabel.
  ///
  /// In en, this message translates to:
  /// **'Mobile number'**
  String get phoneLabel;

  /// No description provided for @phonePrefix.
  ///
  /// In en, this message translates to:
  /// **'+966'**
  String get phonePrefix;

  /// No description provided for @phoneHint.
  ///
  /// In en, this message translates to:
  /// **'5x xxx xxxx'**
  String get phoneHint;

  /// No description provided for @sendCode.
  ///
  /// In en, this message translates to:
  /// **'Send my code on WhatsApp'**
  String get sendCode;

  /// No description provided for @phoneNotOnList.
  ///
  /// In en, this message translates to:
  /// **'This number isn\'t on your company\'s list. Ask your manager to add you.'**
  String get phoneNotOnList;

  /// No description provided for @otpTitle.
  ///
  /// In en, this message translates to:
  /// **'Enter the code we sent on WhatsApp'**
  String get otpTitle;

  /// No description provided for @otpSentTo.
  ///
  /// In en, this message translates to:
  /// **'Sent to {phone}'**
  String otpSentTo(String phone);

  /// No description provided for @otpHint.
  ///
  /// In en, this message translates to:
  /// **'4-digit code'**
  String get otpHint;

  /// No description provided for @otpWrong.
  ///
  /// In en, this message translates to:
  /// **'That code isn\'t right. {attempts} attempts left.'**
  String otpWrong(String attempts);

  /// No description provided for @otpLocked.
  ///
  /// In en, this message translates to:
  /// **'Too many tries. Wait 15 minutes, then ask for a new code.'**
  String get otpLocked;

  /// No description provided for @otpExpired.
  ///
  /// In en, this message translates to:
  /// **'That code has expired. Ask for a new one.'**
  String get otpExpired;

  /// No description provided for @otpTooMany.
  ///
  /// In en, this message translates to:
  /// **'You\'ve asked for a few codes already. Wait a little before asking again.'**
  String get otpTooMany;

  /// No description provided for @otpResend.
  ///
  /// In en, this message translates to:
  /// **'Send a new code'**
  String get otpResend;

  /// No description provided for @changeNumber.
  ///
  /// In en, this message translates to:
  /// **'Change number'**
  String get changeNumber;

  /// No description provided for @devCodeHint.
  ///
  /// In en, this message translates to:
  /// **'Development build: the code is 1234'**
  String get devCodeHint;

  /// No description provided for @welcomeTitle.
  ///
  /// In en, this message translates to:
  /// **'Hi {name}'**
  String welcomeTitle(String name);

  /// No description provided for @welcomePromise1.
  ///
  /// In en, this message translates to:
  /// **'One minute a day. Never more.'**
  String get welcomePromise1;

  /// No description provided for @welcomePromise2.
  ///
  /// In en, this message translates to:
  /// **'A question after each reel, so it sticks.'**
  String get welcomePromise2;

  /// No description provided for @welcomePromise3.
  ///
  /// In en, this message translates to:
  /// **'Your answers stay yours. Managers see readiness, not answers.'**
  String get welcomePromise3;

  /// No description provided for @chooseSlot.
  ///
  /// In en, this message translates to:
  /// **'When should your minute arrive?'**
  String get chooseSlot;

  /// No description provided for @slotShiftStart.
  ///
  /// In en, this message translates to:
  /// **'Shift start'**
  String get slotShiftStart;

  /// No description provided for @slotBreak.
  ///
  /// In en, this message translates to:
  /// **'Break'**
  String get slotBreak;

  /// No description provided for @slotShiftEnd.
  ///
  /// In en, this message translates to:
  /// **'Shift end'**
  String get slotShiftEnd;

  /// No description provided for @startFirstMinute.
  ///
  /// In en, this message translates to:
  /// **'Start my first minute'**
  String get startFirstMinute;

  /// No description provided for @todayTitle.
  ///
  /// In en, this message translates to:
  /// **'Today'**
  String get todayTitle;

  /// No description provided for @greeting.
  ///
  /// In en, this message translates to:
  /// **'Good day, {name}'**
  String greeting(String name);

  /// No description provided for @newReelsCount.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =0{no new reels} =1{1 new reel} other{{count} new reels}}'**
  String newReelsCount(int count);

  /// No description provided for @reviewsCount.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =0{no review} =1{1 review} other{{count} reviews}}'**
  String reviewsCount(int count);

  /// No description provided for @aboutSeconds.
  ///
  /// In en, this message translates to:
  /// **'about {seconds} s'**
  String aboutSeconds(int seconds);

  /// No description provided for @streakChip.
  ///
  /// In en, this message translates to:
  /// **'{days}-day streak'**
  String streakChip(int days);

  /// No description provided for @yesterdayLine.
  ///
  /// In en, this message translates to:
  /// **'Yesterday you remembered {remembered} of {total}.'**
  String yesterdayLine(int remembered, int total);

  /// No description provided for @nextReviewOn.
  ///
  /// In en, this message translates to:
  /// **'Next review on {date}.'**
  String nextReviewOn(String date);

  /// No description provided for @startMinute.
  ///
  /// In en, this message translates to:
  /// **'Start today\'s minute'**
  String get startMinute;

  /// No description provided for @continueMinute.
  ///
  /// In en, this message translates to:
  /// **'Continue today\'s minute'**
  String get continueMinute;

  /// No description provided for @emptyTitle.
  ///
  /// In en, this message translates to:
  /// **'Nothing for today'**
  String get emptyTitle;

  /// No description provided for @emptyBody.
  ///
  /// In en, this message translates to:
  /// **'Enjoy your day. Your next reels arrive with your next shift.'**
  String get emptyBody;

  /// No description provided for @doneTitle.
  ///
  /// In en, this message translates to:
  /// **'Done for today'**
  String get doneTitle;

  /// No description provided for @doneBody.
  ///
  /// In en, this message translates to:
  /// **'Your minute is complete. See you at the next shift.'**
  String get doneBody;

  /// No description provided for @openSaved.
  ///
  /// In en, this message translates to:
  /// **'Saved reels'**
  String get openSaved;

  /// No description provided for @todayLoadError.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t load today\'s plan.'**
  String get todayLoadError;

  /// No description provided for @swipeHint.
  ///
  /// In en, this message translates to:
  /// **'Swipe up for the question'**
  String get swipeHint;

  /// No description provided for @swipeHintNext.
  ///
  /// In en, this message translates to:
  /// **'Swipe up for the next reel'**
  String get swipeHintNext;

  /// No description provided for @answerFirstToast.
  ///
  /// In en, this message translates to:
  /// **'Answer first, then swipe up'**
  String get answerFirstToast;

  /// No description provided for @listenOnly.
  ///
  /// In en, this message translates to:
  /// **'Listen only'**
  String get listenOnly;

  /// No description provided for @captions.
  ///
  /// In en, this message translates to:
  /// **'Captions'**
  String get captions;

  /// No description provided for @saveReel.
  ///
  /// In en, this message translates to:
  /// **'Save'**
  String get saveReel;

  /// No description provided for @savedReel.
  ///
  /// In en, this message translates to:
  /// **'Saved'**
  String get savedReel;

  /// No description provided for @whyThis.
  ///
  /// In en, this message translates to:
  /// **'Why am I seeing this'**
  String get whyThis;

  /// No description provided for @remembered.
  ///
  /// In en, this message translates to:
  /// **'Remembered.'**
  String get remembered;

  /// No description provided for @notQuite.
  ///
  /// In en, this message translates to:
  /// **'Not quite.'**
  String get notQuite;

  /// No description provided for @comesBackOn.
  ///
  /// In en, this message translates to:
  /// **'Comes back on {date}'**
  String comesBackOn(String date);

  /// No description provided for @reviewChip.
  ///
  /// In en, this message translates to:
  /// **'Asked again · day {day}'**
  String reviewChip(int day);

  /// No description provided for @reviewLabel.
  ///
  /// In en, this message translates to:
  /// **'Review'**
  String get reviewLabel;

  /// No description provided for @sourceLine.
  ///
  /// In en, this message translates to:
  /// **'Source: {citation}'**
  String sourceLine(String citation);

  /// No description provided for @presenterLine.
  ///
  /// In en, this message translates to:
  /// **'with {name}'**
  String presenterLine(String name);

  /// No description provided for @answerSavedOffline.
  ///
  /// In en, this message translates to:
  /// **'Answer saved. You\'ll see the result when you\'re back online.'**
  String get answerSavedOffline;

  /// No description provided for @doneRemembered.
  ///
  /// In en, this message translates to:
  /// **'Remembered {remembered} of {total}'**
  String doneRemembered(int remembered, int total);

  /// No description provided for @streakLine.
  ///
  /// In en, this message translates to:
  /// **'{days}-day streak'**
  String streakLine(int days);

  /// No description provided for @freezeNote.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =0{No streak freeze left this week. Fridays never count.} =1{1 streak freeze left for a day you miss. Fridays never count.} other{{count} streak freezes left for days you miss. Fridays never count.}}'**
  String freezeNote(int count);

  /// No description provided for @backToWork.
  ///
  /// In en, this message translates to:
  /// **'Back to work'**
  String get backToWork;

  /// No description provided for @nothingMore.
  ///
  /// In en, this message translates to:
  /// **'That\'s all for today.'**
  String get nothingMore;

  /// No description provided for @unitLabel.
  ///
  /// In en, this message translates to:
  /// **'Unit {index}'**
  String unitLabel(int index);

  /// No description provided for @savedTitle.
  ///
  /// In en, this message translates to:
  /// **'Saved'**
  String get savedTitle;

  /// No description provided for @searchHint.
  ///
  /// In en, this message translates to:
  /// **'Search reels'**
  String get searchHint;

  /// No description provided for @allTopics.
  ///
  /// In en, this message translates to:
  /// **'All topics'**
  String get allTopics;

  /// No description provided for @savedOnly.
  ///
  /// In en, this message translates to:
  /// **'Saved only'**
  String get savedOnly;

  /// No description provided for @noResults.
  ///
  /// In en, this message translates to:
  /// **'No reels match.'**
  String get noResults;

  /// No description provided for @askForReel.
  ///
  /// In en, this message translates to:
  /// **'Ask for a reel'**
  String get askForReel;

  /// No description provided for @askForReelTitle.
  ///
  /// In en, this message translates to:
  /// **'What would help you at work?'**
  String get askForReelTitle;

  /// No description provided for @askForReelHint.
  ///
  /// In en, this message translates to:
  /// **'for example: a refund without a receipt'**
  String get askForReelHint;

  /// No description provided for @askForReelAnon.
  ///
  /// In en, this message translates to:
  /// **'Your manager sees the request, not your name.'**
  String get askForReelAnon;

  /// No description provided for @askSent.
  ///
  /// In en, this message translates to:
  /// **'Thanks. Your request is in.'**
  String get askSent;

  /// No description provided for @durationSeconds.
  ///
  /// In en, this message translates to:
  /// **'{seconds} s'**
  String durationSeconds(int seconds);

  /// No description provided for @progressTitle.
  ///
  /// In en, this message translates to:
  /// **'Progress'**
  String get progressTitle;

  /// No description provided for @rememberedHeadline.
  ///
  /// In en, this message translates to:
  /// **'You remember {pct}%'**
  String rememberedHeadline(int pct);

  /// No description provided for @rememberedCount.
  ///
  /// In en, this message translates to:
  /// **'{count} things remembered'**
  String rememberedCount(int count);

  /// No description provided for @weeklyTitle.
  ///
  /// In en, this message translates to:
  /// **'Week by week'**
  String get weeklyTitle;

  /// No description provided for @byTopicTitle.
  ///
  /// In en, this message translates to:
  /// **'By topic'**
  String get byTopicTitle;

  /// No description provided for @milestonesTitle.
  ///
  /// In en, this message translates to:
  /// **'Milestones'**
  String get milestonesTitle;

  /// No description provided for @longestStreak.
  ///
  /// In en, this message translates to:
  /// **'Longest streak: {days} days'**
  String longestStreak(int days);

  /// No description provided for @reached.
  ///
  /// In en, this message translates to:
  /// **'Reached'**
  String get reached;

  /// No description provided for @notYetReached.
  ///
  /// In en, this message translates to:
  /// **'Not yet'**
  String get notYetReached;

  /// No description provided for @settingsTitle.
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get settingsTitle;

  /// No description provided for @language.
  ///
  /// In en, this message translates to:
  /// **'Language'**
  String get language;

  /// No description provided for @langAr.
  ///
  /// In en, this message translates to:
  /// **'العربية'**
  String get langAr;

  /// No description provided for @langEn.
  ///
  /// In en, this message translates to:
  /// **'English'**
  String get langEn;

  /// No description provided for @langUr.
  ///
  /// In en, this message translates to:
  /// **'اردو'**
  String get langUr;

  /// No description provided for @langBn.
  ///
  /// In en, this message translates to:
  /// **'বাংলা'**
  String get langBn;

  /// No description provided for @deliverySlotSetting.
  ///
  /// In en, this message translates to:
  /// **'When your minute arrives'**
  String get deliverySlotSetting;

  /// No description provided for @channelSetting.
  ///
  /// In en, this message translates to:
  /// **'Where it arrives'**
  String get channelSetting;

  /// No description provided for @channelApp.
  ///
  /// In en, this message translates to:
  /// **'App'**
  String get channelApp;

  /// No description provided for @channelWhatsapp.
  ///
  /// In en, this message translates to:
  /// **'WhatsApp'**
  String get channelWhatsapp;

  /// No description provided for @channelBoth.
  ///
  /// In en, this message translates to:
  /// **'Both'**
  String get channelBoth;

  /// No description provided for @captionsSetting.
  ///
  /// In en, this message translates to:
  /// **'Captions on by default'**
  String get captionsSetting;

  /// No description provided for @remindersSetting.
  ///
  /// In en, this message translates to:
  /// **'Reminders'**
  String get remindersSetting;

  /// No description provided for @streakFreezeSetting.
  ///
  /// In en, this message translates to:
  /// **'Streak freeze'**
  String get streakFreezeSetting;

  /// No description provided for @streakFreezeBody.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =0{None left this week.} =1{1 left this week. It covers a day you miss.} other{{count} left this week. They cover days you miss.}}'**
  String streakFreezeBody(int count);

  /// No description provided for @extraReelsSetting.
  ///
  /// In en, this message translates to:
  /// **'Extra reels'**
  String get extraReelsSetting;

  /// No description provided for @extraReelsBody.
  ///
  /// In en, this message translates to:
  /// **'One more new reel a day. Still under two minutes.'**
  String get extraReelsBody;

  /// No description provided for @pauseSetting.
  ///
  /// In en, this message translates to:
  /// **'Pause for two weeks'**
  String get pauseSetting;

  /// No description provided for @pauseBody.
  ///
  /// In en, this message translates to:
  /// **'Deliveries and your streak stop until you\'re back.'**
  String get pauseBody;

  /// No description provided for @pausedUntil.
  ///
  /// In en, this message translates to:
  /// **'Paused until {date}'**
  String pausedUntil(String date);

  /// No description provided for @resumeNow.
  ///
  /// In en, this message translates to:
  /// **'Resume now'**
  String get resumeNow;

  /// No description provided for @privacyTitle.
  ///
  /// In en, this message translates to:
  /// **'What {company} can see'**
  String privacyTitle(String company);

  /// No description provided for @managerSees.
  ///
  /// In en, this message translates to:
  /// **'Your manager sees'**
  String get managerSees;

  /// No description provided for @hrSees.
  ///
  /// In en, this message translates to:
  /// **'HR sees'**
  String get hrSees;

  /// No description provided for @nobodySees.
  ///
  /// In en, this message translates to:
  /// **'Nobody sees'**
  String get nobodySees;

  /// No description provided for @downloadData.
  ///
  /// In en, this message translates to:
  /// **'Download everything stored about me'**
  String get downloadData;

  /// No description provided for @exportQueued.
  ///
  /// In en, this message translates to:
  /// **'We\'ll send your data on WhatsApp within a day.'**
  String get exportQueued;

  /// No description provided for @signOut.
  ///
  /// In en, this message translates to:
  /// **'Sign out'**
  String get signOut;

  /// No description provided for @settingsSaved.
  ///
  /// In en, this message translates to:
  /// **'Saved'**
  String get settingsSaved;

  /// No description provided for @teamTitle.
  ///
  /// In en, this message translates to:
  /// **'Team'**
  String get teamTitle;

  /// No description provided for @teamRecall.
  ///
  /// In en, this message translates to:
  /// **'Team recall {pct}%'**
  String teamRecall(int pct);

  /// No description provided for @activeThisWeek.
  ///
  /// In en, this message translates to:
  /// **'{active} of {size} active this week'**
  String activeThisWeek(int active, int size);

  /// No description provided for @ready.
  ///
  /// In en, this message translates to:
  /// **'Ready'**
  String get ready;

  /// No description provided for @notYet.
  ///
  /// In en, this message translates to:
  /// **'Not yet'**
  String get notYet;

  /// No description provided for @notStarted.
  ///
  /// In en, this message translates to:
  /// **'Not started'**
  String get notStarted;

  /// No description provided for @nudge.
  ///
  /// In en, this message translates to:
  /// **'Nudge'**
  String get nudge;

  /// No description provided for @nudgedToday.
  ///
  /// In en, this message translates to:
  /// **'Nudged today'**
  String get nudgedToday;

  /// No description provided for @nudgeSent.
  ///
  /// In en, this message translates to:
  /// **'Reminder sent to {name}'**
  String nudgeSent(String name);

  /// No description provided for @nudgeSkipped.
  ///
  /// In en, this message translates to:
  /// **'{name} was already nudged today'**
  String nudgeSkipped(String name);

  /// No description provided for @recordReel.
  ///
  /// In en, this message translates to:
  /// **'Record a reel'**
  String get recordReel;

  /// No description provided for @privacyFootnote.
  ///
  /// In en, this message translates to:
  /// **'You see readiness per topic. Never answers, saves, or times.'**
  String get privacyFootnote;

  /// No description provided for @usesWhatsapp.
  ///
  /// In en, this message translates to:
  /// **'WhatsApp'**
  String get usesWhatsapp;

  /// No description provided for @recordTitle.
  ///
  /// In en, this message translates to:
  /// **'Record a reel'**
  String get recordTitle;

  /// No description provided for @recordTopic.
  ///
  /// In en, this message translates to:
  /// **'Topic'**
  String get recordTopic;

  /// No description provided for @recordPromptTitle.
  ///
  /// In en, this message translates to:
  /// **'Your three lines'**
  String get recordPromptTitle;

  /// No description provided for @promptLine1.
  ///
  /// In en, this message translates to:
  /// **'Line 1: the one thing to remember'**
  String get promptLine1;

  /// No description provided for @promptLine2.
  ///
  /// In en, this message translates to:
  /// **'Line 2: when it matters'**
  String get promptLine2;

  /// No description provided for @promptLine3.
  ///
  /// In en, this message translates to:
  /// **'Line 3: how to do it'**
  String get promptLine3;

  /// No description provided for @consentLabel.
  ///
  /// In en, this message translates to:
  /// **'I agree that my face and voice appear in this reel for {company} staff.'**
  String consentLabel(String company);

  /// No description provided for @startRecording.
  ///
  /// In en, this message translates to:
  /// **'Start recording'**
  String get startRecording;

  /// No description provided for @stopRecording.
  ///
  /// In en, this message translates to:
  /// **'Stop'**
  String get stopRecording;

  /// No description provided for @secondsLeft.
  ///
  /// In en, this message translates to:
  /// **'{seconds} s left'**
  String secondsLeft(int seconds);

  /// No description provided for @retake.
  ///
  /// In en, this message translates to:
  /// **'Retake'**
  String get retake;

  /// No description provided for @sendToReview.
  ///
  /// In en, this message translates to:
  /// **'Send to review'**
  String get sendToReview;

  /// No description provided for @sending.
  ///
  /// In en, this message translates to:
  /// **'Sending'**
  String get sending;

  /// No description provided for @sentToReview.
  ///
  /// In en, this message translates to:
  /// **'Sent. The studio team will review it.'**
  String get sentToReview;

  /// No description provided for @cameraUnavailable.
  ///
  /// In en, this message translates to:
  /// **'The camera isn\'t available on this device.'**
  String get cameraUnavailable;

  /// No description provided for @recordNeedsConsent.
  ///
  /// In en, this message translates to:
  /// **'Tick the consent box to continue.'**
  String get recordNeedsConsent;

  /// No description provided for @recordNeedsPrompt.
  ///
  /// In en, this message translates to:
  /// **'Write your three lines first.'**
  String get recordNeedsPrompt;

  /// No description provided for @recordingLabel.
  ///
  /// In en, this message translates to:
  /// **'Recording'**
  String get recordingLabel;
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) =>
      <String>['ar', 'en', 'ur'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'ar':
      return AppLocalizationsAr();
    case 'en':
      return AppLocalizationsEn();
    case 'ur':
      return AppLocalizationsUr();
  }

  throw FlutterError(
      'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
      'an issue with the localizations generation tool. Please file an issue '
      'on GitHub with a reproducible sample app and the gen-l10n configuration '
      'that was used.');
}
