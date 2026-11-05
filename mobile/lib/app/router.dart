import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../core/auth/session.dart';
import '../features/auth/presentation/signin_screen.dart';
import '../features/auth/presentation/welcome_screen.dart';
import '../features/player/presentation/play_screen.dart';
import '../features/progress/presentation/progress_screen.dart';
import '../features/record/presentation/record_screen.dart';
import '../features/saved/presentation/saved_screen.dart';
import '../features/settings/presentation/settings_screen.dart';
import '../features/team/presentation/team_screen.dart';
import '../features/today/presentation/done_screen.dart';
import '../features/today/presentation/today_screen.dart';
import 'shell.dart';

abstract final class Routes {
  static const splash = '/splash';
  static const signIn = '/signin';
  static const welcome = '/welcome';
  static const today = '/today';
  static const play = '/today/play';
  static const done = '/today/done';
  static const saved = '/saved';
  static const progress = '/progress';
  static const settings = '/settings';
  static const team = '/team';
  static const record = '/team/record';

  static const managerOnly = {team, record};
}

/// Bridges a Riverpod provider to GoRouter's `refreshListenable`.
class _SessionListenable extends ChangeNotifier {
  _SessionListenable(Ref<Object?> ref) {
    ref.listen<SessionState>(sessionProvider, (_, __) => notifyListeners());
  }
}

final routerProvider = Provider<GoRouter>((ref) {
  final listenable = _SessionListenable(ref);
  ref.onDispose(listenable.dispose);

  return GoRouter(
    initialLocation: Routes.today,
    refreshListenable: listenable,
    redirect: (context, state) {
      final session = ref.read(sessionProvider);
      final loc = state.uri.path;
      final onAuth = loc == Routes.signIn || loc.startsWith('/t/');

      switch (session) {
        case SessionLoading():
          return loc == Routes.splash ? null : Routes.splash;
        case SignedOut():
          return onAuth ? null : Routes.signIn;
        case SignedIn(:final me, :final needsWelcome):
          if (needsWelcome) {
            return loc == Routes.welcome ? null : Routes.welcome;
          }
          if (onAuth || loc == Routes.splash || loc == Routes.welcome) {
            return Routes.today;
          }
          if (Routes.managerOnly.contains(loc) && !me.isManager) {
            return Routes.today;
          }
          return null;
      }
    },
    routes: [
      GoRoute(path: '/', redirect: (_, __) => Routes.today),
      GoRoute(
        path: Routes.splash,
        builder: (_, __) => const _Splash(),
      ),
      // Deep link reelearn://t/<slug>?inv=<token>
      GoRoute(
        path: '/t/:slug',
        redirect: (_, state) {
          final slug = state.pathParameters['slug'] ?? '';
          final inv = state.uri.queryParameters['inv'];
          final q = Uri(
            queryParameters: {'slug': slug, if (inv != null) 'inv': inv},
          ).query;
          return '${Routes.signIn}?$q';
        },
      ),
      GoRoute(
        path: Routes.signIn,
        builder: (_, state) => SignInScreen(
          initialSlug: state.uri.queryParameters['slug'],
          inviteToken: state.uri.queryParameters['inv'],
        ),
      ),
      GoRoute(path: Routes.welcome, builder: (_, __) => const WelcomeScreen()),
      GoRoute(path: Routes.play, builder: (_, __) => const PlayScreen()),
      GoRoute(path: Routes.done, builder: (_, __) => const DoneScreen()),
      GoRoute(path: Routes.record, builder: (_, __) => const RecordScreen()),
      ShellRoute(
        builder: (_, __, child) => AppShell(child: child),
        routes: [
          GoRoute(path: Routes.today, builder: (_, __) => const TodayScreen()),
          GoRoute(path: Routes.saved, builder: (_, __) => const SavedScreen()),
          GoRoute(
            path: Routes.progress,
            builder: (_, __) => const ProgressScreen(),
          ),
          GoRoute(
            path: Routes.settings,
            builder: (_, __) => const SettingsScreen(),
          ),
          GoRoute(path: Routes.team, builder: (_, __) => const TeamScreen()),
        ],
      ),
    ],
  );
});

class _Splash extends StatelessWidget {
  const _Splash();

  @override
  Widget build(BuildContext context) => const Scaffold(
        body: Center(child: SizedBox.square(dimension: 24)),
      );
}
