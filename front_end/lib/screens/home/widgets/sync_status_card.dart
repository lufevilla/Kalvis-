import 'package:flutter/material.dart';

import 'package:front_end/core/app_colors.dart';
import 'package:front_end/core/models/checkin.dart';
import 'surface_card.dart';

class SyncStatusCard extends StatelessWidget {
  const SyncStatusCard({super.key, required this.status});

  final SyncStatus status;

  /// TODO(rust): `status` deve vir de conectividade real com o backend
  /// (health-check periódico ou stream/WebSocket do serviço Rust).
  @override
  Widget build(BuildContext context) {
    return SurfaceCard(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'Internet',
            style: TextStyle(
              fontSize: 16,
              fontWeight: FontWeight.w700,
              color: AppColors.navy,
            ),
          ),
          const SizedBox(height: 12),
          Row(
            children: [
              const Icon(Icons.public, size: 18, color: AppColors.navy),
              const SizedBox(width: 8),
              Text(
                status.label,
                style: const TextStyle(fontSize: 14, color: AppColors.navy),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
