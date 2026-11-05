import 'dart:convert';

import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:reelearn_mobile/core/api/dio_client.dart';
import 'package:reelearn_mobile/core/api/fixtures.dart';
import 'package:reelearn_mobile/core/api/interceptors.dart';
import 'package:reelearn_mobile/core/api/models/models.dart';
import 'package:reelearn_mobile/core/auth/token_store.dart';
import 'package:reelearn_mobile/core/errors/app_error.dart';

/// Scripted HTTP adapter: 401 for a stale token, 200 for the current one.
class _Adapter implements HttpClientAdapter {
  _Adapter({required this.validToken});
  String validToken;

  /// Snapshots taken at fetch time: Dio reuses the same RequestOptions on a
  /// retry, so keeping the object would show the rewritten headers.
  final List<({String path, String? auth, Map<String, dynamic> headers})>
      requests = [];

  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<List<int>>? requestStream,
    Future<void>? cancelFuture,
  ) async {
    final auth = options.headers['Authorization'] as String?;
    requests.add(
      (path: options.path, auth: auth, headers: Map.of(options.headers)),
    );
    final ok = isPublicPath(options.path) || auth == 'Bearer $validToken';
    return ResponseBody.fromString(
      ok
          ? jsonEncode({'ok': true})
          : jsonEncode({
              'type': 'about:blank',
              'title': 'Unauthorized',
              'status': 401,
              'request_id': 'req-1',
            }),
      ok ? 200 : 401,
      headers: {
        Headers.contentTypeHeader: [
          ok ? 'application/json' : 'application/problem+json',
        ],
      },
    );
  }

  @override
  void close({bool force = false}) {}
}

TokenPair pair(String access, String refresh) => TokenPair(
      accessToken: access,
      refreshToken: refresh,
      expiresIn: 900,
      user: Fixtures.sara,
    );

void main() {
  late _Adapter adapter;
  late MemoryTokenStore tokens;
  late Dio dio;
  late int refreshCalls;
  late bool sessionLost;
  late bool refreshShouldFail;

  setUp(() {
    adapter = _Adapter(validToken: 'new');
    tokens = MemoryTokenStore(
      const StoredSession(
        accessToken: 'old',
        refreshToken: 'r1',
        user: Fixtures.sara,
      ),
    );
    refreshCalls = 0;
    sessionLost = false;
    refreshShouldFail = false;
    dio = Dio(
      BaseOptions(
        baseUrl: 'http://api.test/v1',
        validateStatus: (s) => s != null && s < 400,
      ),
    )..httpClientAdapter = adapter;
    dio.interceptors.add(
      AuthInterceptor(
        dio: dio,
        tokens: tokens,
        refreshCall: (rt) async {
          refreshCalls++;
          expect(rt, 'r1');
          if (refreshShouldFail) throw const ApiError(status: 401);
          return pair('new', 'r2');
        },
        onSessionLost: () => sessionLost = true,
      ),
    );
  });

  test('a 401 refreshes once, rotates the stored pair and retries', () async {
    final res = await dio.get<Map<String, dynamic>>('/me');
    expect(res.statusCode, 200);
    expect(refreshCalls, 1);
    expect(adapter.requests.length, 2);
    expect(adapter.requests.first.auth, 'Bearer old');
    expect(adapter.requests.last.auth, 'Bearer new');
    final stored = await tokens.read();
    expect(stored?.accessToken, 'new');
    expect(stored?.refreshToken, 'r2');
    expect(sessionLost, isFalse);
  });

  test('concurrent 401s share one refresh', () async {
    await Future.wait([
      dio.get<dynamic>('/me'),
      dio.get<dynamic>('/me/today'),
      dio.get<dynamic>('/me/progress'),
    ]);
    expect(refreshCalls, 1);
    expect(
      adapter.requests.where((r) => r.auth == 'Bearer new').length,
      3,
    );
  });

  test('a failed refresh clears the session and surfaces the 401', () async {
    refreshShouldFail = true;
    await expectLater(
      dio.get<dynamic>('/me'),
      throwsA(
        isA<DioException>()
            .having((e) => e.response?.statusCode, 'status', 401),
      ),
    );
    expect(refreshCalls, 1);
    expect(sessionLost, isTrue);
    expect(await tokens.read(), isNull);
  });

  test('public auth paths carry no token and never refresh', () async {
    await dio
        .post<dynamic>('/auth/otp/request', data: {'tenant_slug': 'alwadi'});
    expect(
      adapter.requests.single.headers.containsKey('Authorization'),
      isFalse,
    );
    expect(refreshCalls, 0);
  });

  test('mapDioError turns problem+json into ApiError with the request id', () {
    final req = RequestOptions(path: '/me');
    final e = DioException(
      requestOptions: req,
      type: DioExceptionType.badResponse,
      response: Response<dynamic>(
        requestOptions: req,
        statusCode: 423,
        data: {
          'type': 'x',
          'title': 'Locked',
          'status': 423,
          'request_id': 'r-9',
        },
      ),
    );
    final mapped = mapDioError(e);
    expect(mapped, isA<ApiError>());
    expect((mapped as ApiError).isLocked, isTrue);
    expect(mapped.requestId, 'r-9');
    expect(
      mapDioError(
        DioException(
          requestOptions: req,
          type: DioExceptionType.connectionError,
        ),
      ),
      isA<NetworkError>(),
    );
  });

  test('idempotency and request-id interceptors add headers', () async {
    final d = Dio(BaseOptions(baseUrl: 'http://api.test/v1'))
      ..httpClientAdapter = _Adapter(validToken: 'x')
      ..interceptors.addAll([RequestIdInterceptor(), IdempotencyInterceptor()]);
    await d.post<dynamic>('/auth/otp/request', data: {});
    final a = d.httpClientAdapter as _Adapter;
    expect(a.requests.single.headers['X-Request-Id'], isNotEmpty);
    expect(a.requests.single.headers.containsKey('Idempotency-Key'), isFalse);
  });
}
