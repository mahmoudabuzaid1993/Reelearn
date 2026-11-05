import 'package:dio/dio.dart';

import '../auth/token_store.dart';
import '../errors/app_error.dart';
import 'interceptors.dart';
import 'models/models.dart';

/// Builds the one Dio the app talks through.
Dio buildDio({
  required String baseUrl,
  required TokenStore tokens,
  void Function()? onSessionLost,
}) {
  final options = BaseOptions(
    baseUrl: baseUrl,
    connectTimeout: const Duration(seconds: 10),
    receiveTimeout: const Duration(seconds: 20),
    sendTimeout: const Duration(seconds: 20),
    headers: const {'Accept': 'application/json, application/problem+json'},
    // Let interceptors see 4xx/5xx; mapping happens in [mapDioError].
    validateStatus: (s) => s != null && s < 400,
  );
  final dio = Dio(options);
  final plain = Dio(options);

  Future<TokenPair> refreshCall(String refreshToken) async {
    final res = await plain.post<Json>(
      '/auth/refresh',
      data: {'refresh_token': refreshToken},
    );
    return TokenPair.fromJson(res.data ?? const {});
  }

  dio.interceptors.addAll([
    RequestIdInterceptor(),
    IdempotencyInterceptor(),
    AuthInterceptor(
      dio: dio,
      tokens: tokens,
      refreshCall: refreshCall,
      onSessionLost: onSessionLost,
    ),
  ]);
  return dio;
}

/// Translates a Dio failure into the one error family the app understands.
AppError mapDioError(DioException e) {
  switch (e.type) {
    case DioExceptionType.connectionTimeout:
    case DioExceptionType.sendTimeout:
    case DioExceptionType.receiveTimeout:
    case DioExceptionType.transformTimeout:
    case DioExceptionType.connectionError:
      return NetworkError(e.message ?? e.type.name);
    case DioExceptionType.badCertificate:
      return const UnexpectedError('bad certificate');
    case DioExceptionType.cancel:
      return const UnexpectedError('cancelled');
    case DioExceptionType.badResponse:
    case DioExceptionType.unknown:
      final res = e.response;
      if (res == null) return NetworkError(e.message ?? 'no response');
      final data = res.data;
      final problem =
          data is Map<String, dynamic> ? Problem.fromJson(data) : null;
      final requestId = problem?.requestId ??
          res.headers.value('x-request-id') ??
          e.requestOptions.headers['X-Request-Id'] as String?;
      return ApiError(
        status: res.statusCode ?? 0,
        problem: problem,
        requestId: requestId,
      );
  }
}
