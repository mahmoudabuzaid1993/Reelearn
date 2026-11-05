import 'package:flutter/material.dart';
import 'package:video_player/video_player.dart';

import '../../../app/theme.dart';
import 'reel_card_view.dart';

/// Real media for a reel card. Plays while active, pauses when not, reports
/// progress for the 90 % event, and hides the picture in listen-only mode.
Widget videoReelMedia(BuildContext context, ReelMediaRequest req) =>
    _VideoReelMedia(req: req);

class _VideoReelMedia extends StatefulWidget {
  const _VideoReelMedia({required this.req});
  final ReelMediaRequest req;

  @override
  State<_VideoReelMedia> createState() => _VideoReelMediaState();
}

class _VideoReelMediaState extends State<_VideoReelMedia>
    with WidgetsBindingObserver {
  VideoPlayerController? _controller;
  bool _failed = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _init();
  }

  Future<void> _init() async {
    final url = Uri.tryParse(widget.req.reel.videoUrl);
    if (url == null) {
      setState(() => _failed = true);
      return;
    }
    final c = VideoPlayerController.networkUrl(url);
    _controller = c;
    try {
      await c.initialize();
      c.addListener(_tick);
      if (!mounted) return;
      setState(() {});
      if (widget.req.isActive) await c.play();
    } on Object {
      if (mounted) setState(() => _failed = true);
    }
  }

  void _tick() {
    final c = _controller;
    if (c == null || !c.value.isInitialized) return;
    widget.req.onProgress(
      c.value.position.inMilliseconds,
      c.value.duration.inMilliseconds,
    );
  }

  @override
  void didUpdateWidget(covariant _VideoReelMedia old) {
    super.didUpdateWidget(old);
    final c = _controller;
    if (c == null || !c.value.isInitialized) return;
    if (widget.req.isActive && !old.req.isActive) {
      c.play();
    } else if (!widget.req.isActive && old.req.isActive) {
      c.pause();
    }
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    final c = _controller;
    if (c == null || !c.value.isInitialized) return;
    // Backgrounded: pause. Resumed: continue the same card where it was.
    if (state == AppLifecycleState.paused ||
        state == AppLifecycleState.inactive) {
      c.pause();
    } else if (state == AppLifecycleState.resumed && widget.req.isActive) {
      c.play();
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _controller?.removeListener(_tick);
    _controller?.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final c = _controller;
    if (_failed || c == null || !c.value.isInitialized) {
      return placeholderReelMedia(context, widget.req);
    }
    if (widget.req.listenOnly) {
      return placeholderReelMedia(context, widget.req);
    }
    return ColoredBox(
      color: Brand.night,
      child: Center(
        child: AspectRatio(
          aspectRatio: c.value.aspectRatio,
          child: VideoPlayer(c),
        ),
      ),
    );
  }
}
