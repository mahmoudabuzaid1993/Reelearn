import 'dart:async';
import 'dart:io';

import 'package:camera/camera.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../../app/l10n/gen/app_localizations.dart';
import '../../../app/theme.dart';
import '../../../core/api/fixtures.dart';
import '../../../core/api/reelearn_api.dart';
import '../../../core/auth/session.dart';
import '../../../core/errors/app_error.dart';
import '../../../core/providers.dart';
import '../domain/record_state.dart';

/// Topics an SME can record for. The manager payload carries topic ids; we
/// reuse the team's topics when loaded, else the seed's three.
const _fallbackTopics = {
  Fixtures.topicCustomerService: 'Customer service',
  Fixtures.topicStoreSafety: 'Store safety',
  Fixtures.topicAml: 'AML',
};

class RecordScreen extends ConsumerStatefulWidget {
  const RecordScreen({super.key});

  @override
  ConsumerState<RecordScreen> createState() => _RecordScreenState();
}

class _RecordScreenState extends ConsumerState<RecordScreen>
    with WidgetsBindingObserver {
  RecordState _state = const RecordState();
  CameraController? _camera;
  bool _cameraUnavailable = false;
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    unawaited(_initCamera());
  }

  Future<void> _initCamera() async {
    try {
      final cams = await availableCameras();
      if (cams.isEmpty) throw StateError('no camera');
      final front = cams.firstWhere(
        (c) => c.lensDirection == CameraLensDirection.front,
        orElse: () => cams.first,
      );
      final c = CameraController(front, ResolutionPreset.high);
      await c.initialize();
      if (!mounted) return;
      setState(() => _camera = c);
    } on Object {
      if (mounted) setState(() => _cameraUnavailable = true);
    }
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState s) {
    if (s == AppLifecycleState.paused &&
        _state.phase == RecordPhase.recording) {
      unawaited(_stop());
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _timer?.cancel();
    _camera?.dispose();
    super.dispose();
  }

  void _set(RecordState s) => setState(() => _state = s);

  void _start() {
    if (!_state.canStart) return;
    _set(_state.beginCountdown());
    _timer?.cancel();
    _timer = Timer.periodic(const Duration(seconds: 1), (_) async {
      final before = _state.phase;
      _set(_state.tickCountdown());
      if (before == RecordPhase.countdown &&
          _state.phase == RecordPhase.recording) {
        _timer?.cancel();
        await _beginRecording();
      }
    });
  }

  Future<void> _beginRecording() async {
    final c = _camera;
    if (c != null) {
      try {
        await c.startVideoRecording();
      } on Object catch (e) {
        _set(_state.copyWith(phase: RecordPhase.failed, error: e.toString()));
        return;
      }
    }
    _timer = Timer.periodic(const Duration(seconds: 1), (_) async {
      _set(_state.tickRecording());
      if (_state.phase == RecordPhase.review) {
        _timer?.cancel();
        await _finishRecording();
      }
    });
  }

  Future<void> _stop() async {
    _timer?.cancel();
    await _finishRecording();
  }

  Future<void> _finishRecording() async {
    final c = _camera;
    String? path;
    if (c != null && c.value.isRecordingVideo) {
      try {
        final file = await c.stopVideoRecording();
        path = file.path;
      } on Object catch (e) {
        _set(_state.copyWith(phase: RecordPhase.failed, error: e.toString()));
        return;
      }
    }
    _set(_state.stop(path));
  }

  Future<void> _send() async {
    final path = _state.filePath;
    final topicId = _state.topicId;
    if (path == null || topicId == null) return;
    _set(_state.copyWith(phase: RecordPhase.uploading, clearError: true));
    final api = ref.read(apiProvider);
    try {
      final bytes = await File(path).readAsBytes();
      final slot = await api.createRecording(
        RecordingRequest(
          topicId: topicId,
          title: _state.lines.first.trim(),
          mime: 'video/mp4',
          bytes: bytes.length,
          consentConfirmed: _state.consent,
        ),
      );
      await api.uploadToSlot(slot, bytes);
      await api.submitRecording(slot.id);
      _set(_state.copyWith(phase: RecordPhase.sent));
    } on AppError catch (e) {
      _set(_state.copyWith(phase: RecordPhase.failed, error: e.toString()));
    } on FileSystemException catch (e) {
      _set(_state.copyWith(phase: RecordPhase.failed, error: e.message));
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final text = Theme.of(context).textTheme;
    final company = ref.watch(meProvider)?.tenant.name ?? '';
    final s = _state;

    return Scaffold(
      appBar: AppBar(
        title: Text(l.recordTitle),
        leading: IconButton(
          icon: const Icon(Icons.close),
          tooltip: l.close,
          onPressed: () => context.pop(),
        ),
      ),
      body: switch (s.phase) {
        RecordPhase.setup => _Setup(
            state: s,
            company: company,
            cameraUnavailable: _cameraUnavailable,
            onTopic: (id) => _set(s.copyWith(topicId: id)),
            onLine: (i, v) => _set(s.setLine(i, v)),
            onConsent: (v) => _set(s.copyWith(consent: v)),
            onStart: _start,
          ),
        RecordPhase.countdown || RecordPhase.recording => _Stage(
            state: s,
            camera: _camera,
            onStop: _stop,
          ),
        RecordPhase.review => _Review(
            state: s,
            onRetake: () => _set(s.retake()),
            onSend: _send,
          ),
        RecordPhase.uploading =>
          Center(child: Text(l.sending, style: text.titleMedium)),
        RecordPhase.sent => Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Icon(Icons.check_circle, color: Brand.mint, size: 40),
                const SizedBox(height: 12),
                Text(l.sentToReview, style: text.titleMedium),
                const Spacer(),
                FilledButton(
                  onPressed: () => context.pop(),
                  child: Text(l.close),
                ),
              ],
            ),
          ),
        RecordPhase.failed => Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(l.errorGeneric, style: text.titleMedium),
                if (s.error != null) ...[
                  const SizedBox(height: 8),
                  Text(
                    s.error!,
                    style: text.bodySmall?.copyWith(color: Brand.slate),
                  ),
                ],
                const SizedBox(height: 16),
                OutlinedButton(
                  onPressed: () => _set(s.retake()),
                  child: Text(l.retake),
                ),
              ],
            ),
          ),
      },
    );
  }
}

class _Setup extends StatelessWidget {
  const _Setup({
    required this.state,
    required this.company,
    required this.cameraUnavailable,
    required this.onTopic,
    required this.onLine,
    required this.onConsent,
    required this.onStart,
  });

  final RecordState state;
  final String company;
  final bool cameraUnavailable;
  final ValueChanged<String> onTopic;
  final void Function(int, String) onLine;
  final ValueChanged<bool> onConsent;
  final VoidCallback onStart;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final text = Theme.of(context).textTheme;
    final hints = [l.promptLine1, l.promptLine2, l.promptLine3];
    return ListView(
      padding: const EdgeInsets.fromLTRB(20, 8, 20, 24),
      children: [
        if (cameraUnavailable)
          Padding(
            padding: const EdgeInsets.only(bottom: 12),
            child: Text(l.cameraUnavailable, style: text.bodyMedium),
          ),
        Text(l.recordTopic, style: text.titleMedium),
        const SizedBox(height: 8),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            for (final e in _fallbackTopics.entries)
              ChoiceChip(
                key: Key('topic-${e.key}'),
                label: Text(e.value),
                selected: state.topicId == e.key,
                onSelected: (_) => onTopic(e.key),
              ),
          ],
        ),
        const SizedBox(height: 20),
        Text(l.recordPromptTitle, style: text.titleMedium),
        const SizedBox(height: 8),
        for (var i = 0; i < 3; i++)
          Padding(
            padding: const EdgeInsets.only(bottom: 10),
            child: TextField(
              key: Key('line-$i'),
              onChanged: (v) => onLine(i, v),
              decoration: InputDecoration(hintText: hints[i]),
              maxLength: 120,
            ),
          ),
        CheckboxListTile(
          key: const Key('consent'),
          value: state.consent,
          onChanged: (v) => onConsent(v ?? false),
          controlAffinity: ListTileControlAffinity.leading,
          contentPadding: EdgeInsets.zero,
          title: Text(l.consentLabel(company)),
        ),
        const SizedBox(height: 12),
        FilledButton.icon(
          key: const Key('start-recording'),
          onPressed: state.canStart ? onStart : null,
          icon: const Icon(Icons.fiber_manual_record),
          label: Text(l.startRecording),
        ),
        if (!state.consent)
          Padding(
            padding: const EdgeInsets.only(top: 8),
            child: Text(
              l.recordNeedsConsent,
              style: text.bodySmall?.copyWith(color: Brand.slate),
            ),
          )
        else if (!state.promptComplete)
          Padding(
            padding: const EdgeInsets.only(top: 8),
            child: Text(
              l.recordNeedsPrompt,
              style: text.bodySmall?.copyWith(color: Brand.slate),
            ),
          ),
      ],
    );
  }
}

/// Countdown then recording: camera preview with the three lines as a
/// teleprompter and a seconds-left line.
class _Stage extends StatelessWidget {
  const _Stage({
    required this.state,
    required this.camera,
    required this.onStop,
  });
  final RecordState state;
  final CameraController? camera;
  final VoidCallback onStop;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final text = Theme.of(context).textTheme;
    final c = camera;
    return ColoredBox(
      color: Brand.night,
      child: Stack(
        fit: StackFit.expand,
        children: [
          if (c != null && c.value.isInitialized) CameraPreview(c),
          Positioned(
            left: 0,
            right: 0,
            top: 0,
            child: Container(
              color: Brand.night.withValues(alpha: 0.75),
              padding: const EdgeInsets.fromLTRB(20, 16, 20, 16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  for (final line in state.lines)
                    Padding(
                      padding: const EdgeInsets.only(bottom: 6),
                      child: Text(
                        line,
                        style: text.titleLarge?.copyWith(color: Brand.white),
                      ),
                    ),
                ],
              ),
            ),
          ),
          if (state.phase == RecordPhase.countdown)
            Center(
              child: Text(
                '${state.countdown}',
                key: const Key('countdown'),
                style: text.displayLarge?.copyWith(
                  color: Brand.white,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ),
          Positioned(
            left: 0,
            right: 0,
            bottom: 32,
            child: Column(
              children: [
                Text(
                  state.phase == RecordPhase.recording
                      ? '${l.recordingLabel} · ${l.secondsLeft(state.secondsLeft)}'
                      : l.recordingLabel,
                  key: const Key('seconds-left'),
                  style: text.titleMedium?.copyWith(color: Brand.white),
                ),
                const SizedBox(height: 12),
                FilledButton(
                  key: const Key('stop-recording'),
                  onPressed:
                      state.phase == RecordPhase.recording ? onStop : null,
                  style:
                      FilledButton.styleFrom(minimumSize: const Size(160, 52)),
                  child: Text(l.stopRecording),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _Review extends StatelessWidget {
  const _Review({
    required this.state,
    required this.onRetake,
    required this.onSend,
  });
  final RecordState state;
  final VoidCallback onRetake;
  final VoidCallback onSend;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final text = Theme.of(context).textTheme;
    return Padding(
      padding: const EdgeInsets.all(24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            l.secondsLeft(state.secondsLeft),
            style: text.bodySmall?.copyWith(color: Brand.slate),
          ),
          const SizedBox(height: 8),
          Text(state.lines.first, style: text.titleLarge),
          const Spacer(),
          FilledButton(
            key: const Key('send-recording'),
            onPressed: state.filePath == null ? null : onSend,
            child: Text(l.sendToReview),
          ),
          const SizedBox(height: 8),
          OutlinedButton(onPressed: onRetake, child: Text(l.retake)),
        ],
      ),
    );
  }
}
