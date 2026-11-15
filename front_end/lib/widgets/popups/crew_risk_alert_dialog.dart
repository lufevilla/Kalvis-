import 'package:flutter/material.dart';

import '../../core/app_colors.dart';
import '../../mocks/popup_mocks.dart';

/// Dialog de alerta de tripulante em risco.
///
/// Uso sugerido:
///   showDialog(context: context, builder: (_) => const CrewRiskAlertDialog());
class CrewRiskAlertDialog extends StatelessWidget {
  const CrewRiskAlertDialog({super.key, this.data = mockCrewRiskAlert});

  /// MOCK por padrão — passe os dados reais quando o alerta vier do backend.
  final CrewRiskAlertData data;

  @override
  Widget build(BuildContext context) {
    return Dialog(
      backgroundColor: Colors.transparent,
      child: Container(
        width: 360,
        padding: const EdgeInsets.all(24),
        decoration: BoxDecoration(
          color: AppColors.surface,
          borderRadius: BorderRadius.circular(20),
          border: Border.all(color: AppColors.accent, width: 2),
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: 56,
              height: 56,
              decoration: const BoxDecoration(
                color: AppColors.accentSoft,
                shape: BoxShape.circle,
              ),
              child: const Icon(
                Icons.warning_amber_rounded,
                color: AppColors.accent,
                size: 28,
              ),
            ),
            const SizedBox(height: 16),
            const Text(
              'Alerta: Tripulante em Risco',
              textAlign: TextAlign.center,
              style: TextStyle(
                fontSize: 18,
                fontWeight: FontWeight.w700,
                color: AppColors.navy,
              ),
            ),
            const SizedBox(height: 16),
            Container(
              width: double.infinity,
              padding: const EdgeInsets.all(14),
              decoration: BoxDecoration(
                color: AppColors.accentSoft,
                borderRadius: BorderRadius.circular(12),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  _InfoRow(label: 'Nome do Tripulante', value: data.crewName),
                  const SizedBox(height: 10),
                  _InfoRow(label: 'Motivo do Risco', value: data.riskReason),
                ],
              ),
            ),
            const SizedBox(height: 12),
            Align(
              alignment: Alignment.centerLeft,
              child: _InfoRow(
                label: 'Localização da Espaçonave',
                value: data.shipLocation,
              ),
            ),
            const SizedBox(height: 20),
            SizedBox(
              width: double.infinity,
              child: FilledButton(
                onPressed: () {
                  // TODO: registrar reconhecimento do alerta e fechar o dialog
                  Navigator.of(context).maybePop();
                },
                style: FilledButton.styleFrom(
                  backgroundColor: AppColors.navy,
                  padding: const EdgeInsets.symmetric(vertical: 14),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(10),
                  ),
                ),
                child: const Text('Entendido'),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _InfoRow extends StatelessWidget {
  const _InfoRow({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          label,
          style: const TextStyle(
            fontSize: 12,
            fontWeight: FontWeight.w600,
            color: AppColors.navy,
          ),
        ),
        Text(
          value,
          style: const TextStyle(fontSize: 13, color: AppColors.textMuted),
        ),
      ],
    );
  }
}
