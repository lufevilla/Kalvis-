import 'package:flutter/material.dart';
import 'package:front_end/screens/home/widgets/surface_card.dart';

import '../../core/app_colors.dart';
import '../../mocks/popup_mocks.dart';

/// Card "Diagnóstico da IA": achados + recomendações + ações
/// (ouvir em voz alta e ir para o chat).
class AiDiagnosisCard extends StatelessWidget {
  const AiDiagnosisCard({
    super.key,
    this.findings = mockDiagnosisFindings,
    this.recommendations = mockDiagnosisRecommendations,
    this.aiMessage,
    this.onListen,
    this.onGoToChat,
  });

  /// MOCKs por padrão — passe os dados reais quando a IA estiver integrada.
  final List<DiagnosisFindingData> findings;
  final List<String> recommendations;
  final String? aiMessage;

  final VoidCallback? onListen;
  final VoidCallback? onGoToChat;

  @override
  Widget build(BuildContext context) {
    return SurfaceCard(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'Diagnóstico da IA',
            style: TextStyle(
              fontSize: 20,
              fontWeight: FontWeight.w700,
              color: AppColors.navy,
            ),
          ),
          const SizedBox(height: 16),
          if (aiMessage != null && aiMessage!.isNotEmpty) ...[
            Text(
              aiMessage!,
              style: const TextStyle(
                fontSize: 13,
                height: 1.4,
                color: AppColors.textMuted,
              ),
            ),
            const SizedBox(height: 16),
          ],
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(14),
            decoration: BoxDecoration(
              color: AppColors.background,
              borderRadius: BorderRadius.circular(12),
            ),
            child: Column(
              children: [
                for (var i = 0; i < findings.length; i++) ...[
                  _FindingRow(data: findings[i]),
                  if (i != findings.length - 1) const SizedBox(height: 10),
                ],
              ],
            ),
          ),
          const SizedBox(height: 20),
          const Text(
            'O que fazer agora',
            style: TextStyle(
              fontSize: 15,
              fontWeight: FontWeight.w700,
              color: AppColors.navy,
            ),
          ),
          const SizedBox(height: 10),
          for (final recommendation in recommendations)
            _BulletLine(text: recommendation),
          const SizedBox(height: 20),
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              OutlinedButton.icon(
                onPressed: () {
                  // TODO: disparar leitura em voz alta (text-to-speech) do diagnóstico
                  onListen?.call();
                },
                icon: const Icon(Icons.volume_up_outlined, size: 18),
                label: const Text('Ouvir'),
                style: OutlinedButton.styleFrom(
                  foregroundColor: AppColors.navy,
                  side: const BorderSide(color: AppColors.border),
                  padding: const EdgeInsets.symmetric(
                    horizontal: 18,
                    vertical: 14,
                  ),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(10),
                  ),
                ),
              ),
              FilledButton.icon(
                onPressed: () {
                  // TODO: navegar para a tela/fluxo de conversa com a IA (Chatbot)
                  onGoToChat?.call();
                },
                icon: const Icon(Icons.arrow_forward, size: 18),
                label: const Text('Ir para conversa'),
                style: FilledButton.styleFrom(
                  backgroundColor: AppColors.navy,
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

class _FindingRow extends StatelessWidget {
  const _FindingRow({required this.data});

  final DiagnosisFindingData data;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        CircleAvatar(
          radius: 16,
          backgroundColor: data.isAlert
              ? AppColors.accentSoft
              : AppColors.border,
          child: Icon(
            data.icon,
            size: 16,
            color: data.isAlert ? AppColors.accent : AppColors.navy,
          ),
        ),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                data.title,
                style: TextStyle(
                  fontSize: 14,
                  fontWeight: FontWeight.w600,
                  color: data.isAlert ? AppColors.accent : AppColors.navy,
                ),
              ),
              Text(
                data.subtitle,
                style: const TextStyle(
                  fontSize: 12,
                  color: AppColors.textSubtle,
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _BulletLine extends StatelessWidget {
  const _BulletLine({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Padding(
            padding: EdgeInsets.only(top: 6, right: 8),
            child: CircleAvatar(radius: 2.5, backgroundColor: AppColors.navy),
          ),
          Expanded(
            child: Text(
              text,
              style: const TextStyle(
                fontSize: 13,
                height: 1.4,
                color: AppColors.textMuted,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
