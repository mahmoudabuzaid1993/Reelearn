import 'package:flutter/material.dart';

import '../../../app/theme.dart';
import '../domain/stack_model.dart';

/// One segment per unit. Upcoming: faint; active: ink; answered: slate;
/// remembered: mint. Mirrors automatically under RTL.
class SegmentsBar extends StatelessWidget {
  const SegmentsBar({required this.states, super.key});
  final List<SegmentState> states;

  static Color colorFor(SegmentState s) => switch (s) {
        SegmentState.upcoming => Brand.white.withValues(alpha: 0.3),
        SegmentState.active => Brand.white,
        SegmentState.answered => Brand.slate,
        SegmentState.remembered => Brand.mint,
      };

  @override
  Widget build(BuildContext context) => Row(
        children: [
          for (var i = 0; i < states.length; i++) ...[
            if (i > 0) const SizedBox(width: 6),
            Expanded(
              child: Semantics(
                label: 'segment-${states[i].name}',
                child: Container(
                  key: Key('segment-$i'),
                  height: 4,
                  decoration: BoxDecoration(
                    color: colorFor(states[i]),
                    borderRadius: BorderRadius.circular(2),
                  ),
                ),
              ),
            ),
          ],
        ],
      );
}
