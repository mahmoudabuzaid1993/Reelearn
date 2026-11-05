import 'dart:async';

import 'package:connectivity_plus/connectivity_plus.dart';

/// Emits `true` when the device regains a network path.
abstract class ConnectivityWatcher {
  Stream<bool> get online;
  Future<bool> isOnline();
}

class PluginConnectivityWatcher implements ConnectivityWatcher {
  PluginConnectivityWatcher([Connectivity? plugin])
      : _plugin = plugin ?? Connectivity();

  final Connectivity _plugin;

  static bool _hasPath(List<ConnectivityResult> results) =>
      results.any((r) => r != ConnectivityResult.none);

  @override
  Stream<bool> get online => _plugin.onConnectivityChanged.map(_hasPath);

  @override
  Future<bool> isOnline() async => _hasPath(await _plugin.checkConnectivity());
}

class FakeConnectivityWatcher implements ConnectivityWatcher {
  FakeConnectivityWatcher({bool initiallyOnline = true})
      : _online = initiallyOnline;

  final StreamController<bool> _controller = StreamController.broadcast();
  bool _online;

  void set({required bool online}) {
    _online = online;
    _controller.add(online);
  }

  @override
  Stream<bool> get online => _controller.stream;

  @override
  Future<bool> isOnline() async => _online;

  Future<void> dispose() => _controller.close();
}
