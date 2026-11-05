import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../core/api/models/models.dart';
import '../../../core/errors/app_error.dart';
import '../../../core/providers.dart';

class LibraryFilter {
  const LibraryFilter({
    this.query = '',
    this.topicName,
    this.savedOnly = false,
  });
  final String query;
  final String? topicName;
  final bool savedOnly;

  LibraryFilter copyWith({
    String? query,
    String? topicName,
    bool clearTopic = false,
    bool? savedOnly,
  }) =>
      LibraryFilter(
        query: query ?? this.query,
        topicName: clearTopic ? null : (topicName ?? this.topicName),
        savedOnly: savedOnly ?? this.savedOnly,
      );
}

final libraryFilterProvider =
    StateProvider<LibraryFilter>((_) => const LibraryFilter());

/// The whole browsable library (one page is plenty for a learner's groups);
/// filtering by topic chip happens client-side on `topic_name` since the
/// learner payload carries names, not ids.
final libraryProvider = FutureProvider<List<ReelCard>>((ref) async {
  final filter = ref.watch(libraryFilterProvider);
  final page = await ref.watch(apiProvider).getLibrary(
        query: filter.query,
        saved: filter.savedOnly ? true : null,
        limit: 100,
      );
  final topic = filter.topicName;
  return topic == null
      ? page.items
      : page.items.where((c) => c.topicName == topic).toList(growable: false);
});

final libraryTopicsProvider = Provider<List<String>>((ref) {
  final items = ref.watch(libraryProvider).valueOrNull ?? const [];
  return {for (final c in items) c.topicName}.toList()..sort();
});

/// Save toggles, optimistic. Returns the new saved state.
class SavedToggles extends Notifier<Map<String, bool>> {
  @override
  Map<String, bool> build() => const {};

  Future<bool> toggle(ReelCard card) async {
    final current = state[card.id] ?? card.saved;
    final next = !current;
    state = {...state, card.id: next};
    try {
      final api = ref.read(apiProvider);
      if (next) {
        await api.saveReel(card.id);
      } else {
        await api.unsaveReel(card.id);
      }
      return next;
    } on AppError {
      state = {...state, card.id: current};
      return current;
    }
  }

  bool isSaved(ReelCard card) => state[card.id] ?? card.saved;
}

final savedTogglesProvider =
    NotifierProvider<SavedToggles, Map<String, bool>>(SavedToggles.new);
