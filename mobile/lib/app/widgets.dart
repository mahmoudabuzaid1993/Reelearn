import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../core/errors/app_error.dart';
import 'l10n/gen/app_localizations.dart';
import 'theme.dart';

/// Human message for an [AppError], in the user's language.
String errorMessage(AppLocalizations l, Object error) {
  if (error is NetworkError) return l.offlineBanner;
  if (error is ApiError) {
    final detail = error.problem?.detail;
    if (detail != null && detail.isNotEmpty) return detail;
  }
  return l.errorGeneric;
}

/// Inline error line with a retry and a "copy request id" affordance.
class ErrorLine extends StatelessWidget {
  const ErrorLine({required this.error, this.onRetry, super.key});

  final Object error;
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final requestId = error is ApiError ? (error as ApiError).requestId : null;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            errorMessage(l, error),
            style: Theme.of(context).textTheme.bodyLarge,
          ),
          const SizedBox(height: 8),
          Row(
            children: [
              if (onRetry != null)
                OutlinedButton(onPressed: onRetry, child: Text(l.retry)),
              if (requestId != null) ...[
                const SizedBox(width: 8),
                TextButton.icon(
                  onPressed: () async {
                    await Clipboard.setData(ClipboardData(text: requestId));
                    if (context.mounted) {
                      ScaffoldMessenger.of(context).showSnackBar(
                        SnackBar(content: Text(l.requestIdCopied)),
                      );
                    }
                  },
                  icon: const Icon(Icons.copy_outlined, size: 18),
                  label: Text(l.copyRequestId),
                ),
              ],
            ],
          ),
        ],
      ),
    );
  }
}

/// Loading placeholder. Never a spinner over content.
class SkeletonBox extends StatelessWidget {
  const SkeletonBox({this.height = 16, this.width, super.key});
  final double height;
  final double? width;

  @override
  Widget build(BuildContext context) => Container(
        height: height,
        width: width,
        decoration: BoxDecoration(
          color: Brand.border,
          borderRadius: BorderRadius.circular(8),
        ),
      );
}

class SkeletonList extends StatelessWidget {
  const SkeletonList({this.rows = 4, super.key});
  final int rows;

  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            for (var i = 0; i < rows; i++) ...[
              const SkeletonBox(height: 20, width: 200),
              const SizedBox(height: 10),
              const SkeletonBox(height: 56),
              const SizedBox(height: 20),
            ],
          ],
        ),
      );
}

class SectionTitle extends StatelessWidget {
  const SectionTitle(this.text, {super.key});
  final String text;

  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.fromLTRB(20, 24, 20, 8),
        child: Text(
          text,
          style: Theme.of(context)
              .textTheme
              .titleMedium
              ?.copyWith(fontWeight: FontWeight.w600),
        ),
      );
}

/// Small pill, e.g. the streak chip.
class Pill extends StatelessWidget {
  const Pill({
    required this.label,
    this.icon,
    this.filled = false,
    this.color,
    super.key,
  });

  final String label;
  final IconData? icon;
  final bool filled;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    final bg = filled ? (color ?? Brand.ink) : Brand.white;
    final fg = filled ? Brand.white : (color ?? Brand.ink);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: bg,
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: filled ? bg : Brand.border),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (icon != null) ...[
            Icon(icon, size: 16, color: fg),
            const SizedBox(width: 6),
          ],
          Text(
            label,
            style: Theme.of(context).textTheme.labelLarge?.copyWith(color: fg),
          ),
        ],
      ),
    );
  }
}
