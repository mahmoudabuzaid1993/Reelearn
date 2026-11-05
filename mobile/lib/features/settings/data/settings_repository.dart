import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../core/api/models/models.dart';
import '../../../core/auth/session.dart';
import '../../../core/errors/app_error.dart';
import '../../../core/providers.dart';

final privacyProvider = FutureProvider<PrivacyStatement>(
  (ref) => ref.watch(apiProvider).getPrivacy(),
);

/// Applies a [MePatch] optimistically and reverts on failure.
class SettingsController extends Notifier<AsyncValue<void>> {
  @override
  AsyncValue<void> build() => const AsyncData(null);

  Future<bool> patch(MePatch patch) async {
    final session = ref.read(sessionProvider.notifier);
    final before = ref.read(meProvider);
    if (before == null) return false;
    state = const AsyncLoading();
    await session.updateMe(before.apply(patch));
    try {
      final updated = await ref.read(apiProvider).patchMe(patch);
      await session.updateMe(updated);
      state = const AsyncData(null);
      return true;
    } on AppError catch (e, st) {
      await session.updateMe(before);
      state = AsyncError(e, st);
      return false;
    }
  }

  Future<bool> requestExport() async {
    try {
      await ref.read(apiProvider).requestExport();
      return true;
    } on AppError {
      return false;
    }
  }
}

final settingsControllerProvider =
    NotifierProvider<SettingsController, AsyncValue<void>>(
  SettingsController.new,
);

/// Two weeks from today, as an ISO date.
String pauseUntilDate([DateTime? now]) {
  final d = (now ?? DateTime.now()).add(const Duration(days: 14));
  return '${d.year}-${d.month.toString().padLeft(2, '0')}-${d.day.toString().padLeft(2, '0')}';
}
