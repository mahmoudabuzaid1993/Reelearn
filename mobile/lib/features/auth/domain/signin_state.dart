import '../../../core/api/models/models.dart';

enum SignInStep { company, phone, code }

enum SignInErrorKind {
  companyNotFound,
  phoneNotOnList,
  wrongCode,
  locked,
  expired,
  tooMany,
  offline,
  generic,
}

class SignInError {
  const SignInError(this.kind, {this.attemptsLeft, this.requestId});
  final SignInErrorKind kind;
  final String? attemptsLeft;
  final String? requestId;
}

class SignInState {
  const SignInState({
    this.step = SignInStep.company,
    this.tenant,
    this.phoneLocal = '',
    this.challenge,
    this.busy = false,
    this.error,
    this.locked = false,
  });

  final SignInStep step;
  final TenantPublic? tenant;

  /// Digits after +966, as typed.
  final String phoneLocal;
  final OtpChallenge? challenge;
  final bool busy;
  final SignInError? error;

  /// Five wrong codes: the challenge is dead until a new one is requested.
  final bool locked;

  String get phoneE164 => '+966${phoneLocal.replaceAll(RegExp(r'\D'), '')}';

  SignInState copyWith({
    SignInStep? step,
    TenantPublic? tenant,
    String? phoneLocal,
    OtpChallenge? challenge,
    bool? busy,
    SignInError? error,
    bool clearError = false,
    bool? locked,
  }) =>
      SignInState(
        step: step ?? this.step,
        tenant: tenant ?? this.tenant,
        phoneLocal: phoneLocal ?? this.phoneLocal,
        challenge: challenge ?? this.challenge,
        busy: busy ?? this.busy,
        error: clearError ? null : (error ?? this.error),
        locked: locked ?? this.locked,
      );
}

/// Saudi mobile: 9 digits starting with 5 after the country code.
bool isValidSaudiMobile(String local) {
  final digits = local.replaceAll(RegExp(r'\D'), '');
  final trimmed = digits.startsWith('0') ? digits.substring(1) : digits;
  return RegExp(r'^5\d{8}$').hasMatch(trimmed);
}

String normaliseLocal(String local) {
  final digits = local.replaceAll(RegExp(r'\D'), '');
  return digits.startsWith('0') ? digits.substring(1) : digits;
}
