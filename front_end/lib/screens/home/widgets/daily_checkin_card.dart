import 'package:flutter/material.dart';

import 'package:front_end/core/app_colors.dart';
import 'surface_card.dart';

class DailyCheckInCard extends StatelessWidget {
  const DailyCheckInCard({super.key, required this.onStartCheckIn});

  final VoidCallback onStartCheckIn;

  @override
  Widget build(BuildContext context) {
    return SurfaceCard(
      padding: const EdgeInsets.all(28),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'DAILY CHECK-IN',
            style: TextStyle(
              color: AppColors.accent,
              fontSize: 12,
              fontWeight: FontWeight.w700,
              letterSpacing: 1,
            ),
          ),
          const SizedBox(height: 8),
          const Text(
            'How are you doing?',
            style: TextStyle(
              fontSize: 28,
              fontWeight: FontWeight.w700,
              color: AppColors.navy,
            ),
          ),
          const SizedBox(height: 12),
          const Text(
            'A short guided check-in on sleep, mood, stress, pain, hydration, '
            'exercise and symptoms. Answer by voice or by text, whichever suits you now.',
            style: TextStyle(
              fontSize: 14,
              height: 1.4,
              color: AppColors.textMuted,
            ),
          ),
          const SizedBox(height: 20),
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              FilledButton.icon(
                onPressed: onStartCheckIn,
                icon: const Icon(Icons.check, size: 18),
                label: const Text('Start check-in'),
                style: FilledButton.styleFrom(
                  backgroundColor: AppColors.navy,
                  foregroundColor: Colors.white,
                  padding: const EdgeInsets.symmetric(
                    horizontal: 18,
                    vertical: 14,
                  ),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(10),
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
