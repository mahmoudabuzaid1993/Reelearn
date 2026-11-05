import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../core/api/models/models.dart';
import '../../../core/providers.dart';

/// Today's plan. Invalidate after the stack closes so the Today screen shows
/// the done state; the API caches the materialised plan on its side.
final todayProvider = FutureProvider<DailyPlan>((ref) {
  return ref.watch(apiProvider).getToday();
});
