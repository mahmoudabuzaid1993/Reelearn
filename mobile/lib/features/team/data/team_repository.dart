import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../core/api/models/models.dart';
import '../../../core/errors/app_error.dart';
import '../../../core/providers.dart';

/// Team readiness plus the nudge action. One nudge per person per day is
/// enforced here (button disabled) and by the API (`skipped_already_nudged`).
class TeamController extends AsyncNotifier<TeamReadiness> {
  @override
  Future<TeamReadiness> build() => ref.watch(apiProvider).getTeam();

  Future<NudgeResult?> nudge(String userId) async {
    final current = state.valueOrNull;
    if (current == null) return null;
    final member = current.members.firstWhere((m) => m.userId == userId);
    if (member.nudgedToday) {
      return const NudgeResult(queued: 0, skippedAlreadyNudged: 1);
    }
    // Optimistic: flip the flag now so a double tap cannot send twice.
    state = AsyncData(_mark(current, userId, nudged: true));
    try {
      final result = await ref.read(apiProvider).nudge([userId]);
      if (result.queued == 0 && result.skippedAlreadyNudged == 0) {
        state = AsyncData(_mark(current, userId, nudged: false));
      }
      return result;
    } on AppError {
      state = AsyncData(_mark(current, userId, nudged: false));
      return null;
    }
  }

  static TeamReadiness _mark(
    TeamReadiness t,
    String userId, {
    required bool nudged,
  }) =>
      t.copyWith(
        members: [
          for (final m in t.members)
            if (m.userId == userId) m.copyWith(nudgedToday: nudged) else m,
        ],
      );
}

final teamProvider = AsyncNotifierProvider<TeamController, TeamReadiness>(
  TeamController.new,
);
