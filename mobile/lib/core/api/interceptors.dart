import 'dart:async';

import 'package:dio/dio.dart';
import 'package:uuid/uuid.dart';

import '../auth/token_store.dart';
import 'models/models.dart';

/// Paths that never carry a Bearer token and never trigger a refresh.
const publicPaths = <String>{
  '/auth/tenant/lookup',
  '/auth/otp/request',
  '/auth/otp/verify',
  '/auth/refresh',
  '/health',
};

bool isPublicPath(String path) => publicPaths.any(path.endsWith);

/// Adds `X-Request-Id` to every request so support can match a phone log
/// line to a server log line.
class RequestIdInterceptor extends Interceptor {
  RequestIdInterceptor([Uuid? uuid]) : _uuid = uuid ?? const Uuid();
  final Uuid _uuid;

  @override
  void onRequest(RequestOptions options, RequestInterceptorHandler handler) {
    options.headers.putIfAbsent('X-Request-Id', _uuid.v4);
    handler.next(options);
  }
}

/// Guarantees an `Idempotency-Key` on the learner's mutating POSTs. Callers
/// normally set it themselves (the offline queue persists the key); this is
/// the safety net for a path that forgot.
class IdempotencyInterceptor extends Interceptor {
  IdempotencyInterceptor([Uuid? uuid]) : _uuid = uuid ?? const Uuid();
  final Uuid _uuid;

  static const _paths = {'/me/watch', '/me/answers'};

  @override
  void onRequest(RequestOptions options, RequestInterceptorHandler handler) {
    if (options.method == 'POST' && _paths.any(options.path.endsWith)) {
      options.headers.putIfAbsent('Idempotency-Key', _uuid.v4);
    }
    handler.next(options);
  }
}

typedef RefreshCall = Future<TokenPair> Function(String refreshToken);

/// Attaches the Bearer token and, on a 401, refreshes once and retries.
///
/// Built on [QueuedInterceptor] so concurrent 401s are handled one at
/// a time: the first refreshes, the rest see the new token and just retry.
class AuthInterceptor extends QueuedInterceptor {
  AuthInterceptor({
    required this.dio,
    required this.tokens,
    required this.refreshCall,
    this.onSessionLost,
  });

  final Dio dio;
  final TokenStore tokens;
  final RefreshCall refreshCall;
  final void Function()? onSessionLost;

  static const _retriedKey = 'reelearn.retried';

  @override
  Future<void> onRequest(
    RequestOptions options,
    RequestInterceptorHandler handler,
  ) async {
    if (!isPublicPath(options.path)) {
      final session = await tokens.read();
      if (session != null) {
        options.headers['Authorization'] = 'Bearer ${session.accessToken}';
      }
    }
    handler.next(options);
  }

  @override
  Future<void> onError(
    DioException err,
    ErrorInterceptorHandler handler,
  ) async {
    final req = err.requestOptions;
    final is401 = err.response?.statusCode == 401;
    if (!is401 || isPublicPath(req.path) || req.extra[_retriedKey] == true) {
      handler.next(err);
      return;
    }

    final session = await tokens.read();
    if (session == null) {
      handler.next(err);
      return;
    }

    final sentWith = req.headers['Authorization'];
    var access = session.accessToken;

    // Another queued request already rotated the token: reuse it.
    if (sentWith == 'Bearer ${session.accessToken}') {
      try {
        final pair = await refreshCall(session.refreshToken);
        await tokens.write(
          session.copyWith(
            accessToken: pair.accessToken,
            refreshToken: pair.refreshToken,
            user: pair.user,
          ),
        );
        access = pair.accessToken;
      } on Object {
        await tokens.clear();
        onSessionLost?.call();
        handler.next(err);
        return;
      }
    }

    try {
      req.headers['Authorization'] = 'Bearer $access';
      req.extra[_retriedKey] = true;
      final response = await dio.fetch<dynamic>(req);
      handler.resolve(response);
    } on DioException catch (e) {
      handler.next(e);
    }
  }
}
