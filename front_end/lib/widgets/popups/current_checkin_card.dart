import 'package:flutter/material.dart';
import 'package:front_end/screens/home/widgets/surface_card.dart';

import '../../core/app_colors.dart';
import '../../mocks/popup_mocks.dart';

/// Card "Check-in Atual": lista vertical com o progresso do check-in
/// em andamento no momento.
class CurrentCheckInCard extends StatelessWidget {
  const CurrentCheckInCard({
    super.key,
    this.items = mockCurrentCheckInItems,
  });

  /// MOCK por padrão — passe os itens reais quando houver dados do backend.
  final List<CurrentCheckInItemData> items;

  @override
  Widget build(BuildContext context) {
    return SurfaceCard(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'Check-in Atual',
            style: TextStyle(
              fontSize: 18,
              fontWeight: FontWeight.w700,
              color: AppColors.navy,
            ),
          ),
          const SizedBox(height: 16),
          for (var i = 0; i < items.length; i++) ...[
            _CheckInRow(data: items[i]),
            if (i != items.length - 1) const SizedBox(height: 10),
          ],
        ],
      ),
    );
  }
}

class _CheckInRow extends StatelessWidget {
  const _CheckInRow({required this.data});

  final CurrentCheckInItemData data;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: AppColors.background,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Row(
        children: [
          CircleAvatar(
            radius: 18,
            backgroundColor: AppColors.border,
            child: Icon(data.icon, size: 18, color: AppColors.navy),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Text(
              data.label,
              style: const TextStyle(
                fontSize: 14,
                fontWeight: FontWeight.w600,
                color: AppColors.navy,
              ),
            ),
          ),
          _StatusBadge(data: data),
        ],
      ),
    );
  }
}

class _StatusBadge extends StatelessWidget {
  const _StatusBadge({required this.data});

  final CurrentCheckInItemData data;

  @override
  Widget build(BuildContext context) {
    if (data.badgeType == CheckInItemBadgeType.inProgress) {
      return Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          color: AppColors.accentSoft,
          borderRadius: BorderRadius.circular(20),
        ),
        child: Text(
          data.statusText,
          style: const TextStyle(
            fontSize: 11,
            fontWeight: FontWeight.w700,
            color: AppColors.accent,
          ),
        ),
      );
    }

    return Text(
      data.statusText,
      style: const TextStyle(fontSize: 13, color: AppColors.textSubtle),
    );
  }
}
