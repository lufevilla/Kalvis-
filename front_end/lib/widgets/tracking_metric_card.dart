import 'package:flutter/material.dart';

import '../../core/app_colors.dart';
import 'package:front_end/core/models/tracking.dart';
import 'range_bar.dart';
import 'waveform_chart.dart';

/// Card genérico de métrica. Mostra só o que a métrica tiver preenchido:
/// descrição, barra de faixa, onda e ação recomendada.
class TrackingMetricCard extends StatelessWidget {
  const TrackingMetricCard({super.key, required this.metric});

  final TrackingMetric metric;

  @override
  Widget build(BuildContext context) {
    final isAlert = metric.status == TrackingStatus.alert;
    final color = _statusColor(metric.status);
    final soft = _statusSoft(metric.status);

    return Container(
      padding: const EdgeInsets.all(20),
      decoration: BoxDecoration(
        color: AppColors.surface,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(
          color: isAlert ? AppColors.accent : AppColors.border,
          width: isAlert ? 1.5 : 1,
        ),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              CircleAvatar(
                radius: 18,
                backgroundColor: isAlert
                    ? AppColors.accentSoft
                    : AppColors.border,
                child: Icon(
                  metric.icon,
                  size: 18,
                  color: isAlert ? AppColors.accent : AppColors.navy,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  metric.title,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                    color: AppColors.navy,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),
          Row(
            crossAxisAlignment: CrossAxisAlignment.end,
            children: [
              Flexible(
                child: Text.rich(
                  TextSpan(
                    children: [
                      TextSpan(
                        text: metric.value,
                        style: const TextStyle(
                          fontSize: 30,
                          fontWeight: FontWeight.w500,
                          color: AppColors.navy,
                        ),
                      ),
                      if (metric.unit.isNotEmpty)
                        TextSpan(
                          text: ' ${metric.unit}',
                          style: const TextStyle(
                            fontSize: 15,
                            color: AppColors.textMuted,
                          ),
                        ),
                    ],
                  ),
                ),
              ),
              const SizedBox(width: 8),
              _StatusBadge(label: metric.statusLabel, color: color, soft: soft),
            ],
          ),
          if (metric.description != null) ...[
            const SizedBox(height: 6),
            Text(
              metric.description!,
              style: const TextStyle(fontSize: 13, color: AppColors.textMuted),
            ),
          ],
          if (metric.rangeBar != null) ...[
            const SizedBox(height: 16),
            RangeBar(data: metric.rangeBar!),
            const SizedBox(height: 8),
            Text(
              metric.rangeBar!.rangeLabel,
              style: const TextStyle(fontSize: 12, color: AppColors.textSubtle),
            ),
          ],
          if (metric.waveform != null) ...[
            const SizedBox(height: 16),
            SizedBox(
              height: 56,
              width: double.infinity,
              child: WaveformChart(
                points: metric.waveform!,
                color: AppColors.navy,
                fillColor: metric.waveformFilled ? AppColors.border : null,
              ),
            ),
          ],
          if (metric.actionText != null) ...[
            const SizedBox(height: 14),
            Text(
              metric.actionText!,
              style: const TextStyle(
                fontSize: 13,
                fontWeight: FontWeight.w600,
                color: AppColors.navy,
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _StatusBadge extends StatelessWidget {
  const _StatusBadge({
    required this.label,
    required this.color,
    required this.soft,
  });

  final String label;
  final Color color;
  final Color soft;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
      decoration: BoxDecoration(
        color: soft,
        borderRadius: BorderRadius.circular(20),
      ),
      child: Text(
        label,
        style: TextStyle(
          fontSize: 12,
          fontWeight: FontWeight.w500,
          color: color,
        ),
      ),
    );
  }
}

Color _statusColor(TrackingStatus status) => switch (status) {
  TrackingStatus.normal => AppColors.success,
  TrackingStatus.attention => AppColors.warning,
  TrackingStatus.alert => AppColors.accent,
};

Color _statusSoft(TrackingStatus status) => switch (status) {
  TrackingStatus.normal => AppColors.successSoft,
  TrackingStatus.attention => AppColors.warningSoft,
  TrackingStatus.alert => AppColors.accentSoft,
};
