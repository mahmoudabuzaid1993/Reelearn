import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../app/l10n/gen/app_localizations.dart';
import '../../../app/theme.dart';
import '../../../core/api/models/models.dart';
import '../../../core/auth/session.dart';
import '../../../core/errors/app_error.dart';
import '../../../core/providers.dart';

/// First open: three promises, delivery slot, "Start my first minute".
class WelcomeScreen extends ConsumerStatefulWidget {
  const WelcomeScreen({super.key});

  @override
  ConsumerState<WelcomeScreen> createState() => _WelcomeScreenState();
}

class _WelcomeScreenState extends ConsumerState<WelcomeScreen> {
  DeliverySlot? _slot;
  bool _busy = false;

  Future<void> _start() async {
    final me = ref.read(meProvider);
    if (me == null) return;
    final slot = _slot ?? me.deliverySlot;
    setState(() => _busy = true);
    try {
      final updated = await ref.read(apiProvider).patchMe(
            MePatch(
              deliverySlot: slot,
              uiLanguage: me.uiLanguage,
              markOpened: true,
            ),
          );
      await ref.read(sessionProvider.notifier).updateMe(updated);
    } on AppError {
      // Offline on first open: keep going, the slot is saved next time.
    }
    await ref.read(sessionProvider.notifier).welcomeDone();
    if (mounted) setState(() => _busy = false);
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final text = Theme.of(context).textTheme;
    final me = ref.watch(meProvider);
    final slot = _slot ?? me?.deliverySlot ?? DeliverySlot.shiftStart;

    return Scaffold(
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(24, 32, 24, 24),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                l.welcomeTitle(me?.displayFirstName ?? ''),
                style:
                    text.headlineMedium?.copyWith(fontWeight: FontWeight.w600),
              ),
              const SizedBox(height: 24),
              _Promise(icon: Icons.timer_outlined, text: l.welcomePromise1),
              _Promise(icon: Icons.help_outline, text: l.welcomePromise2),
              _Promise(icon: Icons.lock_outline, text: l.welcomePromise3),
              const SizedBox(height: 24),
              Text(l.chooseSlot, style: text.titleMedium),
              const SizedBox(height: 12),
              SlotPicker(
                value: slot,
                onChanged: (s) => setState(() => _slot = s),
              ),
              const Spacer(),
              FilledButton(
                key: const Key('start-first-minute'),
                onPressed: _busy ? null : _start,
                child: Text(l.startFirstMinute),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _Promise extends StatelessWidget {
  const _Promise({required this.icon, required this.text});
  final IconData icon;
  final String text;

  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.symmetric(vertical: 10),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(icon, color: Brand.ink),
            const SizedBox(width: 14),
            Expanded(
              child: Text(text, style: Theme.of(context).textTheme.bodyLarge),
            ),
          ],
        ),
      );
}

/// Three chips: shift start, break, shift end. Shared with Settings.
class SlotPicker extends StatelessWidget {
  const SlotPicker({required this.value, required this.onChanged, super.key});
  final DeliverySlot value;
  final ValueChanged<DeliverySlot> onChanged;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    String label(DeliverySlot s) => switch (s) {
          DeliverySlot.shiftStart => l.slotShiftStart,
          DeliverySlot.breakTime => l.slotBreak,
          DeliverySlot.shiftEnd => l.slotShiftEnd,
        };
    return Wrap(
      spacing: 8,
      runSpacing: 8,
      children: [
        for (final s in DeliverySlot.values)
          ChoiceChip(
            key: Key('slot-${s.wire}'),
            label: Text(label(s)),
            selected: value == s,
            onSelected: (_) => onChanged(s),
          ),
      ],
    );
  }
}
