import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../core/auth/session.dart';
import '../../../core/errors/app_error.dart';
import '../data/auth_repository.dart';
import '../domain/signin_state.dart';

class SignInController extends AutoDisposeNotifier<SignInState> {
  @override
  SignInState build() => const SignInState();

  AuthRepository get _repo => ref.read(authRepositoryProvider);

  /// From a deep link `reelearn://t/<slug>?inv=…`: resolve the company and
  /// skip the code entry.
  Future<void> init({String? slug, String? inviteToken}) async {
    if (slug == null && inviteToken == null) return;
    await lookup(slug: slug, inviteToken: inviteToken);
  }

  Future<void> lookup({String? slug, String? inviteToken}) async {
    state = state.copyWith(busy: true, clearError: true);
    try {
      final tenant = await _repo.lookup(slug: slug, inviteToken: inviteToken);
      state =
          state.copyWith(busy: false, tenant: tenant, step: SignInStep.phone);
    } on AppError catch (e) {
      state = state.copyWith(
        busy: false,
        error: _map(e, notFound: SignInErrorKind.companyNotFound),
      );
    }
  }

  void changeCompany() => state = const SignInState();

  void phoneChanged(String local) =>
      state = state.copyWith(phoneLocal: local, clearError: true);

  Future<void> sendCode() async {
    final tenant = state.tenant;
    if (tenant == null || !isValidSaudiMobile(state.phoneLocal)) return;
    state = state.copyWith(busy: true, clearError: true, locked: false);
    try {
      final challenge = await _repo.requestOtp(
        tenant.slug,
        '+966${normaliseLocal(state.phoneLocal)}',
      );
      state = state.copyWith(
        busy: false,
        challenge: challenge,
        step: SignInStep.code,
      );
    } on AppError catch (e) {
      state = state.copyWith(
        busy: false,
        error: _map(e, notFound: SignInErrorKind.phoneNotOnList),
      );
    }
  }

  void changeNumber() => state = state.copyWith(
        step: SignInStep.phone,
        clearError: true,
        locked: false,
      );

  Future<void> verify(String code) async {
    final challenge = state.challenge;
    if (challenge == null || state.locked) return;
    state = state.copyWith(busy: true, clearError: true);
    try {
      final pair = await _repo.verify(challenge.challengeId, code);
      await ref.read(sessionProvider.notifier).signedIn(pair);
      state = state.copyWith(busy: false);
    } on AppError catch (e) {
      final err = _map(e, notFound: SignInErrorKind.expired);
      state = state.copyWith(
        busy: false,
        error: err,
        locked: err.kind == SignInErrorKind.locked,
      );
    }
  }

  static SignInError _map(AppError e, {required SignInErrorKind notFound}) {
    if (e is NetworkError) return const SignInError(SignInErrorKind.offline);
    if (e is ApiError) {
      final id = e.requestId;
      if (e.isLocked) return SignInError(SignInErrorKind.locked, requestId: id);
      if (e.isTooMany) {
        return SignInError(SignInErrorKind.tooMany, requestId: id);
      }
      if (e.isNotFound) return SignInError(notFound, requestId: id);
      if (e.isBadRequest) {
        final detail = e.detail ?? '';
        final m = RegExp(r'(\d+)').firstMatch(detail);
        if (detail.toLowerCase().contains('expired')) {
          return SignInError(SignInErrorKind.expired, requestId: id);
        }
        return SignInError(
          SignInErrorKind.wrongCode,
          attemptsLeft: m?.group(1),
          requestId: id,
        );
      }
      return SignInError(SignInErrorKind.generic, requestId: id);
    }
    return const SignInError(SignInErrorKind.generic);
  }
}

final signInControllerProvider =
    AutoDisposeNotifierProvider<SignInController, SignInState>(
  SignInController.new,
);
