import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:reelearn_mobile/core/api/fixtures.dart';
import 'package:reelearn_mobile/core/auth/session.dart';
import 'package:reelearn_mobile/features/auth/presentation/signin_screen.dart';

import '../../helpers.dart';

Future<void> reachOtp(WidgetTester tester, TestEnv env) async {
  await tester.pumpWidget(env.wrap(const SignInScreen()));
  await tester.pumpAndSettle();
  await tester.enterText(find.byKey(const Key('company-code')), 'alwadi');
  await tester.tap(find.byKey(const Key('company-lookup')));
  await tester.pumpAndSettle();
  expect(find.text('Welcome to Al Wadi Retail'), findsOneWidget);

  await tester.enterText(find.byKey(const Key('phone')), '500000010');
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(const Key('send-code')));
  await tester.pumpAndSettle();
  expect(find.byKey(const Key('otp')), findsOneWidget);
  expect(find.text('Sent to +966500000010'), findsOneWidget);
}

void main() {
  testWidgets('company code, phone, OTP 1234 signs Sara in', (tester) async {
    final env = TestEnv();
    await reachOtp(tester, env);
    await tester.enterText(find.byKey(const Key('otp')), '1234');
    await tester.tap(find.byKey(const Key('verify')));
    await tester.pumpAndSettle();

    final session = containerOf(tester).read(sessionProvider);
    expect(session, isA<SignedIn>());
    expect((session as SignedIn).me.id, Fixtures.saraId);
    expect(session.needsWelcome, isTrue);
    expect(
      (await env.tokens.read())?.refreshToken,
      startsWith('fake-refresh-'),
    );
  });

  testWidgets('wrong code shows attempts left and keeps the form',
      (tester) async {
    final env = TestEnv();
    await reachOtp(tester, env);
    await tester.enterText(find.byKey(const Key('otp')), '0000');
    await tester.tap(find.byKey(const Key('verify')));
    await tester.pumpAndSettle();
    expect(
      find.text("That code isn't right. 4 attempts left."),
      findsOneWidget,
    );
    expect(containerOf(tester).read(sessionProvider), isNot(isA<SignedIn>()));
  });

  testWidgets('five wrong codes lock the challenge', (tester) async {
    final env = TestEnv();
    await reachOtp(tester, env);
    for (var i = 0; i < 5; i++) {
      await tester.enterText(find.byKey(const Key('otp')), '9999');
      await tester.tap(find.byKey(const Key('verify')));
      await tester.pumpAndSettle();
    }
    expect(find.textContaining('Too many tries'), findsOneWidget);
    final verify = tester.widget<FilledButton>(find.byKey(const Key('verify')));
    expect(verify.onPressed, isNull);
    // The right code no longer works on this challenge.
    final otp = tester.widget<TextField>(find.byKey(const Key('otp')));
    expect(otp.enabled, isFalse);
  });

  testWidgets('unknown company and unknown phone show human messages',
      (tester) async {
    final env = TestEnv();
    await tester.pumpWidget(env.wrap(const SignInScreen()));
    await tester.enterText(find.byKey(const Key('company-code')), 'nope');
    await tester.tap(find.byKey(const Key('company-lookup')));
    await tester.pumpAndSettle();
    expect(find.textContaining('No company space'), findsOneWidget);

    await tester.enterText(find.byKey(const Key('company-code')), 'alwadi');
    await tester.tap(find.byKey(const Key('company-lookup')));
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(const Key('phone')), '599999999');
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('send-code')));
    await tester.pumpAndSettle();
    expect(find.textContaining("isn't on your company's list"), findsOneWidget);
  });

  testWidgets('deep link slug skips the company step', (tester) async {
    final env = TestEnv();
    await tester.pumpWidget(
      env.wrap(
        const SignInScreen(initialSlug: 'alwadi', inviteToken: 'inv123'),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Welcome to Al Wadi Retail'), findsOneWidget);
    expect(find.byKey(const Key('phone')), findsOneWidget);
    expect(find.byKey(const Key('company-code')), findsNothing);
  });
}
