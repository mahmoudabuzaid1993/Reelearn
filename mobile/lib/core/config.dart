import 'package:flutter/foundation.dart';

enum ApiMode { fake, http }

/// Build-time configuration, read from `--dart-define`.
///
/// * `API=fake` selects the in-memory [FakeApi]; anything else is HTTP.
/// * `API_BASE=http://host:8080/v1` overrides the base URL. On an Android
///   emulator `localhost` is rewritten to `10.0.2.2` (the host loopback).
class AppConfig {
  const AppConfig({required this.mode, required this.baseUrl});

  factory AppConfig.fromEnvironment() {
    const api = String.fromEnvironment('API', defaultValue: 'http');
    const base = String.fromEnvironment(
      'API_BASE',
      defaultValue: 'http://localhost:8080/v1',
    );
    return AppConfig(
      mode: api == 'fake' ? ApiMode.fake : ApiMode.http,
      baseUrl: resolveBaseUrl(base, defaultTargetPlatform, isWeb: kIsWeb),
    );
  }

  final ApiMode mode;
  final String baseUrl;

  bool get isFake => mode == ApiMode.fake;

  @visibleForTesting
  static String resolveBaseUrl(
    String base,
    TargetPlatform platform, {
    bool isWeb = false,
  }) {
    if (!isWeb && platform == TargetPlatform.android) {
      return base
          .replaceFirst('://localhost', '://10.0.2.2')
          .replaceFirst('://127.0.0.1', '://10.0.2.2');
    }
    return base;
  }
}
