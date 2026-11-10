import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../app/l10n/gen/app_localizations.dart';
import '../../../app/theme.dart';
import '../../../app/widgets.dart';
import '../../../core/api/models/models.dart';
import '../../../core/errors/app_error.dart';
import '../../../core/providers.dart';

/// "Your topics": interest per topic, follow and mute, the personalisation
/// switch, and the subjects the engine thinks the person leans toward.
class MyTopicsScreen extends ConsumerStatefulWidget {
  const MyTopicsScreen({super.key});

  @override
  ConsumerState<MyTopicsScreen> createState() => _MyTopicsScreenState();
}

class _MyTopicsScreenState extends ConsumerState<MyTopicsScreen> {
  MyInterests? _data;
  Object? _error;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() => _error = null);
    try {
      final d = await ref.read(apiProvider).getMyTopics();
      if (mounted) setState(() => _data = d);
    } on Object catch (e) {
      if (mounted) setState(() => _error = e);
    }
  }

  Future<void> _apply(Future<MyInterests> Function() call) async {
    setState(() => _busy = true);
    try {
      final d = await call();
      if (mounted) setState(() => _data = d);
    } on ApiError catch (e) {
      if (mounted && e.status == 422) {
        ScaffoldMessenger.maybeOf(context)?.showSnackBar(
          SnackBar(
            content: Text(AppLocalizations.of(context).topicsMuteRequired),
          ),
        );
      }
    } on Object catch (e) {
      if (mounted) setState(() => _error = e);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final text = Theme.of(context).textTheme;
    final api = ref.read(apiProvider);
    final d = _data;
    return Scaffold(
      appBar: AppBar(title: Text(l.topicsTitle)),
      body: d == null
          ? Center(
              child: _error == null
                  ? const CircularProgressIndicator(color: Brand.ink)
                  : ErrorLine(error: _error!, onRetry: _load),
            )
          : ListView(
              key: const Key('my-topics'),
              padding: const EdgeInsets.only(bottom: 32),
              children: [
                Padding(
                  padding: const EdgeInsets.fromLTRB(20, 12, 20, 8),
                  child: Text(
                    l.topicsSubtitle,
                    style: text.bodyMedium?.copyWith(color: Brand.slate),
                  ),
                ),
                SwitchListTile(
                  key: const Key('personalized-switch'),
                  title: Text(l.topicsPersonalized),
                  subtitle: Text(l.topicsPersonalizedHint),
                  value: d.personalized,
                  onChanged: _busy
                      ? null
                      : (v) => _apply(() => api.setPersonalization(enabled: v)),
                ),
                const Divider(height: 24),
                for (final t in d.topics)
                  _TopicRow(
                    topic: t,
                    busy: _busy,
                    onChanged: (p) =>
                        _apply(() => api.setTopicPreference(t.topicId, p)),
                  ),
                if (d.topTags.isNotEmpty) ...[
                  Padding(
                    padding: const EdgeInsets.fromLTRB(20, 24, 20, 8),
                    child: Text(l.topicsTags, style: text.titleSmall),
                  ),
                  Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 20),
                    child: Wrap(
                      spacing: 8,
                      runSpacing: 8,
                      children: [
                        for (final tag in d.topTags)
                          Chip(
                            label: Text(tag.tag),
                            backgroundColor: Brand.paper,
                            side: const BorderSide(color: Brand.border),
                          ),
                      ],
                    ),
                  ),
                ],
              ],
            ),
    );
  }
}

class _TopicRow extends StatelessWidget {
  const _TopicRow({
    required this.topic,
    required this.busy,
    required this.onChanged,
  });

  final MyTopic topic;
  final bool busy;
  final void Function(TopicPreference?) onChanged;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final text = Theme.of(context).textTheme;
    return Padding(
      key: Key('topic-${topic.topicId}'),
      padding: const EdgeInsets.fromLTRB(20, 8, 20, 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Expanded(child: Text(topic.name, style: text.titleMedium)),
              if (topic.isMandatory)
                Container(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                  decoration: BoxDecoration(
                    color: Brand.ink,
                    borderRadius: BorderRadius.circular(999),
                  ),
                  child: Text(
                    l.topicRequired,
                    style: text.labelSmall?.copyWith(color: Brand.white),
                  ),
                ),
            ],
          ),
          const SizedBox(height: 8),
          Row(
            children: [
              Expanded(
                child: ClipRRect(
                  borderRadius: BorderRadius.circular(4),
                  child: LinearProgressIndicator(
                    value: topic.interest / 100,
                    minHeight: 6,
                    color: Brand.mint,
                    backgroundColor: Brand.border,
                  ),
                ),
              ),
              const SizedBox(width: 12),
              Text(
                l.topicInterest(topic.interest),
                style: text.labelMedium?.copyWith(color: Brand.slate),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            children: [
              ChoiceChip(
                key: Key('follow-${topic.topicId}'),
                label: Text(l.topicFollow),
                selected: topic.preference == TopicPreference.follow,
                onSelected: busy
                    ? null
                    : (on) => onChanged(on ? TopicPreference.follow : null),
              ),
              ChoiceChip(
                key: Key('mute-${topic.topicId}'),
                label: Text(l.topicMute),
                selected: topic.preference == TopicPreference.mute,
                onSelected: busy || topic.isMandatory
                    ? null
                    : (on) => onChanged(on ? TopicPreference.mute : null),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
