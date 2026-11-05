import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../core/auth/session.dart';
import 'l10n/gen/app_localizations.dart';
import 'router.dart';

/// Bottom navigation around the main tabs. The Team tab appears only for
/// managers; the player and the recorder open full-screen above the shell.
class AppShell extends ConsumerWidget {
  const AppShell({required this.child, super.key});
  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final me = ref.watch(meProvider);
    final isManager = me?.isManager ?? false;

    final tabs = <_Tab>[
      _Tab(Routes.today, l.navToday, Icons.today_outlined, Icons.today),
      _Tab(Routes.saved, l.navSaved, Icons.bookmark_outline, Icons.bookmark),
      _Tab(
        Routes.progress,
        l.navProgress,
        Icons.insights_outlined,
        Icons.insights,
      ),
      if (isManager)
        _Tab(Routes.team, l.navTeam, Icons.groups_outlined, Icons.groups),
      _Tab(Routes.settings, l.navSettings, Icons.tune_outlined, Icons.tune),
    ];

    final location = GoRouterState.of(context).uri.path;
    var index = tabs.indexWhere((t) => location.startsWith(t.path));
    if (index < 0) index = 0;

    return Scaffold(
      body: child,
      bottomNavigationBar: NavigationBar(
        selectedIndex: index,
        onDestinationSelected: (i) => context.go(tabs[i].path),
        destinations: [
          for (final t in tabs)
            NavigationDestination(
              icon: Icon(t.icon),
              selectedIcon: Icon(t.selectedIcon),
              label: t.label,
            ),
        ],
      ),
    );
  }
}

class _Tab {
  const _Tab(this.path, this.label, this.icon, this.selectedIcon);
  final String path;
  final String label;
  final IconData icon;
  final IconData selectedIcon;
}
