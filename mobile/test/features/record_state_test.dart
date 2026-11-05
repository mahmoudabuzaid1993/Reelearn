import 'package:flutter_test/flutter_test.dart';
import 'package:reelearn_mobile/features/record/domain/record_state.dart';

void main() {
  test('cannot start without topic, three lines and consent', () {
    var s = const RecordState();
    expect(s.canStart, isFalse);
    s = s
        .copyWith(topicId: 't1')
        .setLine(0, 'a')
        .setLine(1, 'b')
        .setLine(2, 'c');
    expect(s.canStart, isFalse);
    s = s.copyWith(consent: true);
    expect(s.canStart, isTrue);
  });

  test('countdown 3-2-1 then recording, hard stop at 60 s', () {
    var s =
        const RecordState(topicId: 't1', consent: true, lines: ['a', 'b', 'c'])
            .beginCountdown();
    expect(s.phase, RecordPhase.countdown);
    expect(s.countdown, 3);
    s = s.tickCountdown().tickCountdown();
    expect(s.countdown, 1);
    s = s.tickCountdown();
    expect(s.phase, RecordPhase.recording);
    expect(s.secondsLeft, RecordState.maxSeconds);
    for (var i = 0; i < 59; i++) {
      s = s.tickRecording();
    }
    expect(s.phase, RecordPhase.recording);
    expect(s.secondsLeft, 1);
    s = s.tickRecording();
    expect(s.phase, RecordPhase.review);
    expect(s.secondsLeft, 0);
    s = s.retake();
    expect(s.phase, RecordPhase.setup);
    expect(s.secondsLeft, RecordState.maxSeconds);
  });
}
