/// Recorder state machine, independent of the camera plugin so it can be
/// unit-tested. Sixty seconds hard cap.
enum RecordPhase {
  setup,
  countdown,
  recording,
  review,
  uploading,
  sent,
  failed
}

class RecordState {
  const RecordState({
    this.phase = RecordPhase.setup,
    this.topicId,
    this.lines = const ['', '', ''],
    this.consent = false,
    this.countdown = 3,
    this.secondsLeft = maxSeconds,
    this.filePath,
    this.error,
  });

  static const maxSeconds = 60;

  final RecordPhase phase;
  final String? topicId;
  final List<String> lines;
  final bool consent;
  final int countdown;
  final int secondsLeft;
  final String? filePath;
  final String? error;

  bool get promptComplete => lines.every((l) => l.trim().isNotEmpty);
  bool get canStart =>
      phase == RecordPhase.setup &&
      consent &&
      promptComplete &&
      topicId != null;
  int get elapsedSeconds => maxSeconds - secondsLeft;

  RecordState copyWith({
    RecordPhase? phase,
    String? topicId,
    List<String>? lines,
    bool? consent,
    int? countdown,
    int? secondsLeft,
    String? filePath,
    String? error,
    bool clearError = false,
  }) =>
      RecordState(
        phase: phase ?? this.phase,
        topicId: topicId ?? this.topicId,
        lines: lines ?? this.lines,
        consent: consent ?? this.consent,
        countdown: countdown ?? this.countdown,
        secondsLeft: secondsLeft ?? this.secondsLeft,
        filePath: filePath ?? this.filePath,
        error: clearError ? null : (error ?? this.error),
      );

  // Pure transitions.

  RecordState setLine(int i, String v) {
    final next = List<String>.of(lines);
    next[i] = v;
    return copyWith(lines: next);
  }

  RecordState beginCountdown() =>
      copyWith(phase: RecordPhase.countdown, countdown: 3, clearError: true);

  RecordState tickCountdown() {
    if (phase != RecordPhase.countdown) return this;
    if (countdown <= 1) {
      return copyWith(phase: RecordPhase.recording, secondsLeft: maxSeconds);
    }
    return copyWith(countdown: countdown - 1);
  }

  RecordState tickRecording() {
    if (phase != RecordPhase.recording) return this;
    if (secondsLeft <= 1) {
      return copyWith(secondsLeft: 0, phase: RecordPhase.review);
    }
    return copyWith(secondsLeft: secondsLeft - 1);
  }

  RecordState stop(String? path) =>
      copyWith(phase: RecordPhase.review, filePath: path);

  RecordState retake() => copyWith(
        phase: RecordPhase.setup,
        secondsLeft: maxSeconds,
        countdown: 3,
        clearError: true,
      );
}
