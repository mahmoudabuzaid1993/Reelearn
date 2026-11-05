import '../api/models/models.dart';

/// Every failure the UI can see is one of these. Widgets never see Dio.
sealed class AppError implements Exception {
  const AppError();
}

/// The API answered with a problem+json document.
class ApiError extends AppError {
  const ApiError({required this.status, this.problem, this.requestId});

  final int status;
  final Problem? problem;
  final String? requestId;

  bool get isLocked => status == 423;
  bool get isTooMany => status == 429;
  bool get isNotFound => status == 404;
  bool get isConflict => status == 409;
  bool get isUnauthorized => status == 401;
  bool get isBadRequest => status == 400;

  String? get detail => problem?.detail;

  @override
  String toString() => 'ApiError($status, ${problem?.title}, $requestId)';
}

/// No network, DNS failure, timeout. The event is worth retrying later.
class NetworkError extends AppError {
  const NetworkError([this.message = 'offline']);
  final String message;

  @override
  String toString() => 'NetworkError($message)';
}

/// The session is gone (refresh failed). The app returns to sign-in.
class SessionLostError extends AppError {
  const SessionLostError();
}

/// Anything else, kept as a message for the inline error line.
class UnexpectedError extends AppError {
  const UnexpectedError(this.message);
  final String message;

  @override
  String toString() => 'UnexpectedError($message)';
}
