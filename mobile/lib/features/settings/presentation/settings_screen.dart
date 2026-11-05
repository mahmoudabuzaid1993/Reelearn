import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../app/format.dart';
import '../../../app/l10n/gen/app_localizations.dart';
import '../../../app/theme.dart';
import '../../../app/widgets.dart';
import '../../../core/api/models/models.dart';
import '../../../core/auth/session.dart';
import '../../auth/presentation/welcome_screen.dart';
import '../data/settings_repository.dart';

class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final me = ref.watch(meProvider);
    final ctrl = ref.read(settingsControllerProvider.notifier);
    final privacy = ref.watch(privacyProvider);
    final text = Theme.of(context).textTheme;

    ref.listen(settingsControllerProvider, (_, next) {
      if (next.hasError) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(errorMessage(l, next.error!))),
        );
      }
    });

    if (me == null) return const Scaffold();

    Future<void> apply(MePatch p) async {
      final ok = await ctrl.patch(p);
      if (ok && context.mounted) {
        ScaffoldMessenger.of(context)
          ..hideCurrentSnackBar()
          ..showSnackBar(SnackBar(content: Text(l.settingsSaved)));
      }
    }

    return Scaffold(
      appBar: AppBar(title: Text(l.settingsTitle)),
      body: ListView(
        padding: const EdgeInsets.only(bottom: 32),
        children: [
          SectionTitle(l.language),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 20),
            child: Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                for (final (code, label) in [
                  ('ar', l.langAr),
                  ('en', l.langEn),
                  ('ur', l.langUr),
                  ('bn', l.langBn),
                ])
                  ChoiceChip(
                    key: Key('lang-$code'),
                    label: Text(label),
                    selected: me.uiLanguage == code,
                    onSelected: (_) => apply(
                      MePatch(uiLanguage: code, preferredLanguage: code),
                    ),
                  ),
              ],
            ),
          ),
          SectionTitle(l.deliverySlotSetting),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 20),
            child: SlotPicker(
              value: me.deliverySlot,
              onChanged: (s) => apply(MePatch(deliverySlot: s)),
            ),
          ),
          SectionTitle(l.channelSetting),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 20),
            child: Wrap(
              spacing: 8,
              children: [
                for (final (c, label) in [
                  (Channel.app, l.channelApp),
                  (Channel.whatsapp, l.channelWhatsapp),
                  (Channel.both, l.channelBoth),
                ])
                  ChoiceChip(
                    key: Key('channel-${c.wire}'),
                    label: Text(label),
                    selected: me.deliveryChannel == c,
                    onSelected: (_) => apply(MePatch(deliveryChannel: c)),
                  ),
              ],
            ),
          ),
          const SizedBox(height: 12),
          SwitchListTile(
            key: const Key('switch-captions'),
            title: Text(l.captionsSetting),
            value: me.captionsOn,
            onChanged: (v) => apply(MePatch(captionsOn: v)),
          ),
          SwitchListTile(
            key: const Key('switch-reminders'),
            title: Text(l.remindersSetting),
            value: me.remindersOn,
            onChanged: (v) => apply(MePatch(remindersOn: v)),
          ),
          SwitchListTile(
            key: const Key('switch-extra-reels'),
            title: Text(l.extraReelsSetting),
            subtitle: Text(l.extraReelsBody),
            value: me.extraReelsOn,
            onChanged: (v) => apply(MePatch(extraReelsOn: v)),
          ),
          ListTile(
            leading: const Icon(Icons.ac_unit_outlined, color: Brand.ink),
            title: Text(l.streakFreezeSetting),
            subtitle: _FreezeLine(),
          ),
          if (me.pausedUntil != null)
            ListTile(
              key: const Key('paused-tile'),
              leading: const Icon(Icons.pause_circle_outline, color: Brand.ink),
              title:
                  Text(l.pausedUntil(formatIsoDate(context, me.pausedUntil))),
              trailing: TextButton(
                key: const Key('resume'),
                onPressed: () => apply(const MePatch(clearPausedUntil: true)),
                child: Text(l.resumeNow),
              ),
            )
          else
            ListTile(
              key: const Key('pause-tile'),
              leading: const Icon(Icons.pause_circle_outline, color: Brand.ink),
              title: Text(l.pauseSetting),
              subtitle: Text(l.pauseBody),
              onTap: () => apply(MePatch(pausedUntil: pauseUntilDate())),
            ),
          SectionTitle(l.privacyTitle(me.tenant.name)),
          privacy.when(
            loading: () => const SkeletonList(rows: 1),
            error: (e, _) => ErrorLine(
              error: e,
              onRetry: () => ref.invalidate(privacyProvider),
            ),
            data: (p) => Padding(
              padding: const EdgeInsets.symmetric(horizontal: 20),
              child: Card(
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      _PrivacyGroup(title: l.managerSees, items: p.managerSees),
                      _PrivacyGroup(title: l.hrSees, items: p.hrSees),
                      _PrivacyGroup(title: l.nobodySees, items: p.nobodySees),
                    ],
                  ),
                ),
              ),
            ),
          ),
          const SizedBox(height: 8),
          ListTile(
            key: const Key('download-data'),
            leading: const Icon(Icons.download_outlined, color: Brand.ink),
            title: Text(l.downloadData),
            onTap: () async {
              final ok = await ctrl.requestExport();
              if (context.mounted) {
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(content: Text(ok ? l.exportQueued : l.errorGeneric)),
                );
              }
            },
          ),
          ListTile(
            key: const Key('sign-out'),
            leading: const Icon(Icons.logout, color: Brand.ink),
            title: Text(l.signOut),
            onTap: () => ref.read(sessionProvider.notifier).signOut(),
          ),
          Padding(
            padding: const EdgeInsets.fromLTRB(20, 16, 20, 0),
            child: Text(
              '${me.fullName} · ${me.phoneE164 ?? ''}',
              style: text.bodySmall?.copyWith(color: Brand.slate),
            ),
          ),
        ],
      ),
    );
  }
}

class _FreezeLine extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    // The count comes with today's plan stats; fall back to the fixture
    // default (one per week) until the plan loads.
    return Text(l.streakFreezeBody(1));
  }
}

class _PrivacyGroup extends StatelessWidget {
  const _PrivacyGroup({required this.title, required this.items});
  final String title;
  final List<String> items;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(title, style: text.labelLarge?.copyWith(color: Brand.slate)),
          const SizedBox(height: 4),
          for (final i in items) Text('· $i', style: text.bodyMedium),
        ],
      ),
    );
  }
}
