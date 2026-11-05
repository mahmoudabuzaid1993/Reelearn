import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../core/api/models/models.dart';
import '../../../core/providers.dart';

final progressProvider = FutureProvider<Progress>(
  (ref) => ref.watch(apiProvider).getProgress(),
);
