import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../app/l10n/gen/app_localizations.dart';
import '../../../app/theme.dart';
import '../../../app/widgets.dart';
import '../../../core/api/models/models.dart';
import '../../../core/errors/app_error.dart';
import '../../../core/providers.dart';
import '../data/library_repository.dart';

class SavedScreen extends ConsumerStatefulWidget {
  const SavedScreen({super.key});

  @override
  ConsumerState<SavedScreen> createState() => _SavedScreenState();
}

class _SavedScreenState extends ConsumerState<SavedScreen> {
  final _search = TextEditingController();

  @override
  void dispose() {
    _search.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final filter = ref.watch(libraryFilterProvider);
    final library = ref.watch(libraryProvider);
    final topics = ref.watch(libraryTopicsProvider);
    final toggles = ref.watch(savedTogglesProvider.notifier);
    ref.watch(savedTogglesProvider);

    return Scaffold(
      appBar: AppBar(
        title: Text(l.savedTitle),
        actions: [
          TextButton.icon(
            key: const Key('ask-for-reel'),
            onPressed: () => _askForReel(context),
            icon: const Icon(Icons.add_comment_outlined, size: 18),
            label: Text(l.askForReel),
          ),
        ],
      ),
      body: Column(
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(20, 4, 20, 8),
            child: TextField(
              key: const Key('library-search'),
              controller: _search,
              onChanged: (q) => ref.read(libraryFilterProvider.notifier).state =
                  filter.copyWith(query: q),
              decoration: InputDecoration(
                hintText: l.searchHint,
                prefixIcon: const Icon(Icons.search),
              ),
            ),
          ),
          SizedBox(
            height: 48,
            child: ListView(
              scrollDirection: Axis.horizontal,
              padding: const EdgeInsets.symmetric(horizontal: 20),
              children: [
                FilterChip(
                  key: const Key('chip-saved-only'),
                  label: Text(l.savedOnly),
                  selected: filter.savedOnly,
                  onSelected: (v) => ref
                      .read(libraryFilterProvider.notifier)
                      .state = filter.copyWith(savedOnly: v),
                ),
                const SizedBox(width: 8),
                ChoiceChip(
                  label: Text(l.allTopics),
                  selected: filter.topicName == null,
                  onSelected: (_) => ref
                      .read(libraryFilterProvider.notifier)
                      .state = filter.copyWith(clearTopic: true),
                ),
                for (final t in topics) ...[
                  const SizedBox(width: 8),
                  ChoiceChip(
                    key: Key('chip-topic-$t'),
                    label: Text(t),
                    selected: filter.topicName == t,
                    onSelected: (_) => ref
                        .read(libraryFilterProvider.notifier)
                        .state = filter.copyWith(topicName: t),
                  ),
                ],
              ],
            ),
          ),
          Expanded(
            child: library.when(
              loading: () => const SkeletonList(rows: 3),
              error: (e, _) => ErrorLine(
                error: e,
                onRetry: () => ref.invalidate(libraryProvider),
              ),
              data: (items) => items.isEmpty
                  ? Center(
                      child: Text(
                        l.noResults,
                        style: Theme.of(context)
                            .textTheme
                            .bodyLarge
                            ?.copyWith(color: Brand.slate),
                      ),
                    )
                  : GridView.builder(
                      padding: const EdgeInsets.fromLTRB(20, 8, 20, 24),
                      gridDelegate:
                          const SliverGridDelegateWithMaxCrossAxisExtent(
                        maxCrossAxisExtent: 220,
                        mainAxisSpacing: 12,
                        crossAxisSpacing: 12,
                        childAspectRatio: 0.72,
                      ),
                      itemCount: items.length,
                      itemBuilder: (_, i) => ReelTile(
                        card: items[i],
                        saved: toggles.isSaved(items[i]),
                        onToggleSave: () => toggles.toggle(items[i]),
                      ),
                    ),
            ),
          ),
        ],
      ),
    );
  }

  Future<void> _askForReel(BuildContext context) async {
    final l = AppLocalizations.of(context);
    final api = ref.read(apiProvider);
    final controller = TextEditingController();
    final sent = await showModalBottomSheet<bool>(
      context: context,
      isScrollControlled: true,
      backgroundColor: Brand.white,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(Brand.radius)),
      ),
      builder: (ctx) => Padding(
        padding: EdgeInsets.fromLTRB(
          20,
          20,
          20,
          20 + MediaQuery.viewInsetsOf(ctx).bottom,
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(l.askForReelTitle, style: Theme.of(ctx).textTheme.titleMedium),
            const SizedBox(height: 4),
            Text(
              l.askForReelAnon,
              style: Theme.of(ctx)
                  .textTheme
                  .bodySmall
                  ?.copyWith(color: Brand.slate),
            ),
            const SizedBox(height: 12),
            TextField(
              key: const Key('ask-text'),
              controller: controller,
              maxLines: 3,
              maxLength: 500,
              decoration: InputDecoration(hintText: l.askForReelHint),
            ),
            const SizedBox(height: 8),
            FilledButton(
              key: const Key('ask-send'),
              onPressed: () async {
                final text = controller.text.trim();
                if (text.length < 3) return;
                try {
                  await api.requestReel(text);
                  if (ctx.mounted) Navigator.of(ctx).pop(true);
                } on AppError {
                  if (ctx.mounted) Navigator.of(ctx).pop(false);
                }
              },
              child: Text(l.send),
            ),
          ],
        ),
      ),
    );
    controller.dispose();
    if (sent == true && context.mounted) {
      ScaffoldMessenger.of(context)
          .showSnackBar(SnackBar(content: Text(l.askSent)));
    }
  }
}

class ReelTile extends StatelessWidget {
  const ReelTile({
    required this.card,
    required this.saved,
    required this.onToggleSave,
    super.key,
  });

  final ReelCard card;
  final bool saved;
  final VoidCallback onToggleSave;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final text = Theme.of(context).textTheme;
    return Card(
      key: Key('reel-tile-${card.id}'),
      clipBehavior: Clip.antiAlias,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Expanded(
            child: Stack(
              fit: StackFit.expand,
              children: [
                const ColoredBox(color: Brand.night),
                Center(
                  child: Icon(
                    Icons.play_circle_outline,
                    color: Brand.white.withValues(alpha: 0.6),
                    size: 36,
                  ),
                ),
                PositionedDirectional(
                  top: 4,
                  end: 4,
                  child: IconButton(
                    key: Key('save-toggle-${card.id}'),
                    tooltip: saved ? l.savedReel : l.saveReel,
                    onPressed: onToggleSave,
                    icon: Icon(
                      saved ? Icons.bookmark : Icons.bookmark_outline,
                      color: Brand.white,
                    ),
                  ),
                ),
                PositionedDirectional(
                  bottom: 6,
                  end: 8,
                  child: Text(
                    l.durationSeconds((card.durationMs / 1000).round()),
                    style: text.labelSmall?.copyWith(color: Brand.white),
                  ),
                ),
              ],
            ),
          ),
          Padding(
            padding: const EdgeInsets.all(10),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  card.title,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: text.bodyMedium?.copyWith(fontWeight: FontWeight.w600),
                ),
                const SizedBox(height: 2),
                Text(
                  card.topicName,
                  style: text.bodySmall?.copyWith(color: Brand.slate),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
