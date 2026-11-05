import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../app/l10n/gen/app_localizations.dart';
import '../../../app/theme.dart';
import '../../../core/config.dart';
import '../../../core/providers.dart';
import '../domain/signin_state.dart';
import 'signin_controller.dart';

class SignInScreen extends ConsumerStatefulWidget {
  const SignInScreen({super.key, this.initialSlug, this.inviteToken});
  final String? initialSlug;
  final String? inviteToken;

  @override
  ConsumerState<SignInScreen> createState() => _SignInScreenState();
}

class _SignInScreenState extends ConsumerState<SignInScreen> {
  final _company = TextEditingController();
  final _phone = TextEditingController();
  final _code = TextEditingController();

  @override
  void initState() {
    super.initState();
    if (widget.initialSlug != null || widget.inviteToken != null) {
      _company.text = widget.initialSlug ?? '';
      WidgetsBinding.instance.addPostFrameCallback((_) {
        ref.read(signInControllerProvider.notifier).init(
              slug: widget.initialSlug,
              inviteToken: widget.inviteToken,
            );
      });
    }
  }

  @override
  void dispose() {
    _company.dispose();
    _phone.dispose();
    _code.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final state = ref.watch(signInControllerProvider);
    final ctrl = ref.read(signInControllerProvider.notifier);
    final text = Theme.of(context).textTheme;
    final isFake = ref.watch(appConfigProvider).mode == ApiMode.fake;

    return Scaffold(
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.fromLTRB(24, 32, 24, 24),
          children: [
            Text(
              l.appName,
              style: text.titleMedium
                  ?.copyWith(color: Brand.coral, fontWeight: FontWeight.w700),
            ),
            const SizedBox(height: 8),
            Text(
              l.signInTitle,
              style: text.headlineMedium?.copyWith(fontWeight: FontWeight.w600),
            ),
            const SizedBox(height: 24),
            if (state.tenant != null)
              Padding(
                padding: const EdgeInsets.only(bottom: 16),
                child: Row(
                  children: [
                    Expanded(
                      child: Text(
                        l.companyFound(state.tenant!.name),
                        key: const Key('company-found'),
                        style: text.titleMedium,
                      ),
                    ),
                    if (state.step != SignInStep.code)
                      TextButton(
                        onPressed: ctrl.changeCompany,
                        child: Text(l.changeCompany),
                      ),
                  ],
                ),
              ),
            switch (state.step) {
              SignInStep.company => _CompanyStep(
                  controller: _company,
                  busy: state.busy,
                  onSubmit: () => ctrl.lookup(slug: _company.text.trim()),
                ),
              SignInStep.phone => _PhoneStep(
                  controller: _phone,
                  busy: state.busy,
                  valid: isValidSaudiMobile(state.phoneLocal),
                  onChanged: ctrl.phoneChanged,
                  onSubmit: ctrl.sendCode,
                ),
              SignInStep.code => _CodeStep(
                  controller: _code,
                  busy: state.busy,
                  locked: state.locked,
                  phone: state.phoneE164,
                  onSubmit: () => ctrl.verify(_code.text.trim()),
                  onResend: () {
                    _code.clear();
                    ctrl.sendCode();
                  },
                  onChangeNumber: () {
                    _code.clear();
                    ctrl.changeNumber();
                  },
                ),
            },
            if (state.error != null)
              Padding(
                padding: const EdgeInsets.only(top: 16),
                child: Text(
                  _errorText(l, state.error!),
                  key: const Key('signin-error'),
                  style: text.bodyMedium?.copyWith(color: Brand.ink),
                ),
              ),
            if (isFake && state.step == SignInStep.code)
              Padding(
                padding: const EdgeInsets.only(top: 16),
                child: Text(
                  l.devCodeHint,
                  style: text.bodySmall?.copyWith(color: Brand.slate),
                ),
              ),
          ],
        ),
      ),
    );
  }

  static String _errorText(AppLocalizations l, SignInError e) =>
      switch (e.kind) {
        SignInErrorKind.companyNotFound => l.companyNotFound,
        SignInErrorKind.phoneNotOnList => l.phoneNotOnList,
        SignInErrorKind.wrongCode => l.otpWrong(e.attemptsLeft ?? '?'),
        SignInErrorKind.locked => l.otpLocked,
        SignInErrorKind.expired => l.otpExpired,
        SignInErrorKind.tooMany => l.otpTooMany,
        SignInErrorKind.offline => l.offlineBanner,
        SignInErrorKind.generic => l.errorGeneric,
      };
}

class _CompanyStep extends StatelessWidget {
  const _CompanyStep({
    required this.controller,
    required this.busy,
    required this.onSubmit,
  });
  final TextEditingController controller;
  final bool busy;
  final VoidCallback onSubmit;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        TextField(
          key: const Key('company-code'),
          controller: controller,
          autocorrect: false,
          textInputAction: TextInputAction.done,
          onSubmitted: (_) => onSubmit(),
          decoration: InputDecoration(
            labelText: l.companyCodeLabel,
            hintText: l.companyCodeHint,
          ),
        ),
        const SizedBox(height: 16),
        FilledButton(
          key: const Key('company-lookup'),
          onPressed: busy ? null : onSubmit,
          child: Text(l.companyLookup),
        ),
      ],
    );
  }
}

class _PhoneStep extends StatelessWidget {
  const _PhoneStep({
    required this.controller,
    required this.busy,
    required this.valid,
    required this.onChanged,
    required this.onSubmit,
  });
  final TextEditingController controller;
  final bool busy;
  final bool valid;
  final ValueChanged<String> onChanged;
  final VoidCallback onSubmit;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Directionality(
          textDirection: TextDirection.ltr,
          child: TextField(
            key: const Key('phone'),
            controller: controller,
            keyboardType: TextInputType.phone,
            inputFormatters: [FilteringTextInputFormatter.digitsOnly],
            onChanged: onChanged,
            onSubmitted: (_) => onSubmit(),
            decoration: InputDecoration(
              labelText: l.phoneLabel,
              hintText: l.phoneHint,
              prefixText: '${l.phonePrefix} ',
            ),
          ),
        ),
        const SizedBox(height: 16),
        FilledButton.icon(
          key: const Key('send-code'),
          onPressed: busy || !valid ? null : onSubmit,
          icon: const Icon(Icons.chat_outlined),
          label: Text(l.sendCode),
        ),
      ],
    );
  }
}

class _CodeStep extends StatelessWidget {
  const _CodeStep({
    required this.controller,
    required this.busy,
    required this.locked,
    required this.phone,
    required this.onSubmit,
    required this.onResend,
    required this.onChangeNumber,
  });
  final TextEditingController controller;
  final bool busy;
  final bool locked;
  final String phone;
  final VoidCallback onSubmit;
  final VoidCallback onResend;
  final VoidCallback onChangeNumber;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final text = Theme.of(context).textTheme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(l.otpTitle, style: text.titleMedium),
        const SizedBox(height: 4),
        Text(
          l.otpSentTo(phone),
          style: text.bodySmall?.copyWith(color: Brand.slate),
        ),
        const SizedBox(height: 16),
        Directionality(
          textDirection: TextDirection.ltr,
          child: TextField(
            key: const Key('otp'),
            controller: controller,
            enabled: !locked,
            keyboardType: TextInputType.number,
            inputFormatters: [
              FilteringTextInputFormatter.digitsOnly,
              LengthLimitingTextInputFormatter(8),
            ],
            onSubmitted: (_) => onSubmit(),
            decoration: InputDecoration(hintText: l.otpHint),
            style: text.headlineSmall?.copyWith(letterSpacing: 8),
          ),
        ),
        const SizedBox(height: 16),
        FilledButton(
          key: const Key('verify'),
          onPressed: busy || locked ? null : onSubmit,
          child: Text(l.continueLabel),
        ),
        const SizedBox(height: 8),
        Row(
          children: [
            TextButton(
              key: const Key('resend'),
              onPressed: busy ? null : onResend,
              child: Text(l.otpResend),
            ),
            const Spacer(),
            TextButton(onPressed: onChangeNumber, child: Text(l.changeNumber)),
          ],
        ),
      ],
    );
  }
}
