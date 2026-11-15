import 'package:flutter/material.dart';
import 'package:front_end/core/app_colors.dart';
import 'package:front_end/core/models/checkin.dart';
import 'surface_card.dart';

class CheckInStatusCard extends StatelessWidget {
  const CheckInStatusCard({super.key, required this.summary});

  final DailyCheckInSummary summary;

  @override
  Widget build(BuildContext context) {
    return SurfaceCard(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              const Text(
                'Check-in status',
                style: TextStyle(
                  fontSize: 18,
                  fontWeight: FontWeight.w700,
                  color: AppColors.navy,
                ),
              ),
              Text(
                '${summary.answeredCount} of ${summary.totalCount} answered today',
                style: const TextStyle(
                  fontSize: 12,
                  color: AppColors.textSubtle,
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),
          LayoutBuilder(
            builder: (context, constraints) {
              final columns = constraints.maxWidth >= 560 ? 3 : 2;
              return GridView.count(
                crossAxisCount: columns,
                crossAxisSpacing: 12,
                mainAxisSpacing: 12,
                childAspectRatio: 2.8,
                shrinkWrap: true,
                physics: const NeverScrollableScrollPhysics(),
                children: [
                  for (final item in summary.categories)
                    _StatusItem(status: item),
                ],
              );
            },
          ),
        ],
      ),
    );
  }
}

class _StatusItem extends StatelessWidget {
  const _StatusItem({required this.status});

  final CheckInCategoryStatus status;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: AppColors.surface,
        border: Border.all(color: AppColors.border),
        borderRadius: BorderRadius.circular(12),
      ),
      child: Row(
        children: [
          CircleAvatar(
            radius: 18,
            backgroundColor: AppColors.border,
            child: Icon(
              _iconFor(status.category),
              size: 18,
              color: AppColors.navy,
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  status.label,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                    color: AppColors.navy,
                  ),
                ),
                // TODO(rust): trocar pelo status real retornado pela API
                Text(
                  status.answered ? 'Answered' : 'Not yet answered',
                  style: const TextStyle(
                    fontSize: 12,
                    color: AppColors.textSubtle,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

IconData _iconFor(CheckInCategory category) => switch (category) {
  CheckInCategory.sleep => Icons.bedtime_outlined,
  CheckInCategory.mood => Icons.sentiment_satisfied_alt_outlined,
  CheckInCategory.stress => Icons.monitor_heart_outlined,
  CheckInCategory.pain => Icons.bolt_outlined,
  CheckInCategory.hydration => Icons.water_drop_outlined,
  CheckInCategory.exercise => Icons.fitness_center_outlined,
  CheckInCategory.symptoms => Icons.thermostat_outlined,
};
