/// Small, strict helpers for hand-written `fromJson` constructors.
typedef Json = Map<String, dynamic>;

String asString(Json j, String key, {String fallback = ''}) {
  final v = j[key];
  return v is String ? v : fallback;
}

String? asStringOrNull(Json j, String key) {
  final v = j[key];
  return v is String ? v : null;
}

int asInt(Json j, String key, {int fallback = 0}) {
  final v = j[key];
  if (v is int) return v;
  if (v is num) return v.toInt();
  return fallback;
}

int? asIntOrNull(Json j, String key) {
  final v = j[key];
  if (v is int) return v;
  if (v is num) return v.toInt();
  return null;
}

bool asBool(Json j, String key, {bool fallback = false}) {
  final v = j[key];
  return v is bool ? v : fallback;
}

List<Json> asJsonList(Json j, String key) {
  final v = j[key];
  if (v is! List) return const [];
  return v.whereType<Json>().toList(growable: false);
}

List<String> asStringList(Json j, String key) {
  final v = j[key];
  if (v is! List) return const [];
  return v.whereType<String>().toList(growable: false);
}

Json? asJsonOrNull(Json j, String key) {
  final v = j[key];
  return v is Json ? v : null;
}

DateTime? asDateOrNull(Json j, String key) {
  final v = j[key];
  if (v is! String) return null;
  return DateTime.tryParse(v);
}

String dateOnly(DateTime d) {
  final m = d.month.toString().padLeft(2, '0');
  final day = d.day.toString().padLeft(2, '0');
  return '${d.year}-$m-$day';
}
