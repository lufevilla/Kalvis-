import 'package:flutter/material.dart';
import 'package:front_end/screens/home/widgets/surface_card.dart';
import 'package:front_end/widgets/popups/checkin_popup.dart';

import '../../core/app_colors.dart';
import 'checkin_popup_dialog.dart';

/// Card que convida a pessoa a abrir o pop-up de check-in e mostra
/// quantas categorias já foram respondidas.
class CheckInEntryCard extends StatelessWidget {
  const CheckInEntryCard({
    super.key,
    required this.answeredCount,
    required this.totalCount,
    required this.onAnswersSubmitted,
  });

  final int answeredCount;
  final int totalCount;

  /// Chamado quando o pop-up é salvo, com as respostas escolhidas.
  final ValueChanged<List<CheckInAnswer>> onAnswersSubmitted;

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
            'exercise and symptoms. Pick what fits best — it only takes a minute.',
            style: TextStyle(
              fontSize: 14,
              height: 1.4,
              color: AppColors.textMuted,
            ),
          ),
          const SizedBox(height: 16),
          Text(
            '$answeredCount of $totalCount answered today',
            style: const TextStyle(fontSize: 12, color: AppColors.textSubtle),
          ),
          const SizedBox(height: 16),
          FilledButton.icon(
            onPressed: () async {
              // O próprio CheckInPopupDialog fecha ao salvar, devolvendo
              // as respostas como resultado do showDialog.
              final answers = await showDialog<List<CheckInAnswer>>(
                context: context,
                builder: (_) => const CheckInPopupDialog(),
              );
              if (answers != null) {
                onAnswersSubmitted(answers);
              }
            },
            icon: const Icon(Icons.check, size: 18),
            label: const Text('Start check-in'),
            style: FilledButton.styleFrom(
              backgroundColor: AppColors.navy,
              foregroundColor: Colors.white,
              padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 14),
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(10),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
