import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../app/l10n/gen/app_localizations.dart';
import '../../../app/theme.dart';
import '../../../core/api/models/models.dart';
import '../../../core/errors/app_error.dart';
import '../../../core/providers.dart';

/// "Send to a colleague": search, pick up to ten, an optional note.
Future<void> showShareSheet(
  BuildContext context,
  WidgetRef ref,
  ReelPlayable reel,
) async {
  final sent = await showModalBottomSheet<int>(
    context: context,
    isScrollControlled: true,
    backgroundColor: Brand.white,
    shape: const RoundedRectangleBorder(
      borderRadius: BorderRadius.vertical(top: Radius.circular(Brand.radius)),
    ),
    builder: (_) => ShareSheet(reel: reel),
  );
  if (sent != null && sent > 0 && context.mounted) {
    ScaffoldMessenger.maybeOf(context)?.showSnackBar(
      SnackBar(content: Text(AppLocalizations.of(context).shareSent(sent))),
    );
  }
}

class ShareSheet extends ConsumerStatefulWidget {
  const ShareSheet({required this.reel, super.key});

  final ReelPlayable reel;

  @override
  ConsumerState<ShareSheet> createState() => _ShareSheetState();
}

class _ShareSheetState extends ConsumerState<ShareSheet> {
  static const maxRecipients = 10;
  final _note = TextEditingController();
  final Set<String> _picked = {};
  String _query = '';
  Future<List<Colleague>>? _people;
  bool _sending = false;
  Object? _error;

  @override
  void initState() {
    super.initState();
    _people = ref.read(apiProvider).getColleagues();
  }

  @override
  void dispose() {
    _note.dispose();
    super.dispose();
  }

  void _search(String q) {
    setState(() {
      _query = q;
      _people = ref.read(apiProvider).getColleagues(query: q);
    });
  }

  Future<void> _send() async {
    setState(() {
      _sending = true;
      _error = null;
    });
    try {
      final sent = await ref.read(apiProvider).shareReel(
            widget.reel.id,
            _picked.toList(),
            note: _note.text,
          );
      if (mounted) Navigator.of(context).pop(sent);
    } on AppError catch (e) {
      if (mounted) {
        setState(() {
          _sending = false;
          _error = e;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final text = Theme.of(context).textTheme;
    return SafeArea(
      child: Padding(
        padding: EdgeInsets.fromLTRB(
          20,
          16,
          20,
          16 + MediaQuery.viewInsetsOf(context).bottom,
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(l.shareTitle, style: text.titleMedium),
            const SizedBox(height: 4),
            Text(
              widget.reel.title,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: text.bodySmall?.copyWith(color: Brand.slate),
            ),
            const SizedBox(height: 12),
            TextField(
              key: const Key('share-search'),
              decoration: InputDecoration(
                labelText: l.shareSearch,
                prefixIcon: const Icon(Icons.search),
              ),
              onChanged: _search,
            ),
            const SizedBox(height: 8),
            ConstrainedBox(
              constraints: const BoxConstraints(maxHeight: 260),
              child: FutureBuilder<List<Colleague>>(
                key: ValueKey(_query),
                future: _people,
                builder: (context, snap) {
                  if (snap.connectionState != ConnectionState.done) {
                    return const Padding(
                      padding: EdgeInsets.all(16),
                      child: Center(child: CircularProgressIndicator()),
                    );
                  }
                  final people = snap.data ?? const <Colleague>[];
                  if (people.isEmpty) {
                    return Padding(
                      padding: const EdgeInsets.all(16),
                      child: Text(l.shareNone),
                    );
                  }
                  return ListView(
                    shrinkWrap: true,
                    children: [
                      for (final c in people)
                        CheckboxListTile(
                          key: Key('share-${c.id}'),
                          value: _picked.contains(c.id),
                          onChanged: (v) => setState(() {
                            if (v ?? false) {
                              if (_picked.length < maxRecipients) {
                                _picked.add(c.id);
                              }
                            } else {
                              _picked.remove(c.id);
                            }
                          }),
                          title: Text(c.fullName),
                          subtitle: (c.jobTitle ?? c.location) == null
                              ? null
                              : Text((c.jobTitle ?? c.location)!),
                          controlAffinity: ListTileControlAffinity.leading,
                        ),
                    ],
                  );
                },
              ),
            ),
            const SizedBox(height: 8),
            TextField(
              key: const Key('share-note'),
              controller: _note,
              maxLength: 280,
              decoration: InputDecoration(labelText: l.shareNote),
            ),
            if (_error != null)
              Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: Text(
                  l.errorGeneric,
                  style: text.bodySmall?.copyWith(color: Brand.ink),
                ),
              ),
            FilledButton(
              key: const Key('share-send'),
              onPressed: _picked.isEmpty || _sending ? null : _send,
              child: Text(l.shareSendCount(_picked.length)),
            ),
          ],
        ),
      ),
    );
  }
}
