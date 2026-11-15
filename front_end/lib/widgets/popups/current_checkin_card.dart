import 'package:flutter/material.dart';
import 'package:front_end/screens/home/widgets/surface_card.dart';

import '../../core/app_colors.dart';
import '../../core/models/checkin.dart';

/// Card de check-in que a pessoa de fato preenche.
///
/// Cada categoria vira uma linha com campo de texto livre. Ao digitar,
/// `answered` passa a `true` automaticamente. O botão "Salvar check-in"
/// dispara `onSubmit` com o resumo atualizado.
class CheckInFormCard extends StatefulWidget {
  const CheckInFormCard({
    super.key,
    required this.initialSummary,
    this.onSubmit,
  });

  /// Estado inicial (hoje, vem de mocks/home_mocks.dart).
  final DailyCheckInSummary initialSummary;

  /// TODO(rust): enviar o resumo preenchido para o backend, ex.:
  ///   POST /checkins/today  com  summary.toJson()
  final ValueChanged<DailyCheckInSummary>? onSubmit;

  @override
  State<CheckInFormCard> createState() => _CheckInFormCardState();
}

class _CheckInFormCardState extends State<CheckInFormCard> {
  late List<CheckInCategoryStatus> _items;
  late final List<TextEditingController> _controllers;

  @override
  void initState() {
    super.initState();
    _items = List.of(widget.initialSummary.categories);
    _controllers = _items
        .map((item) => TextEditingController(text: item.value ?? ''))
        .toList();
  }

  @override
  void dispose() {
    for (final controller in _controllers) {
      controller.dispose();
    }
    super.dispose();
  }

  void _onChanged(int index, String text) {
    setState(() {
      _items[index] = _items[index].copyWith(
        answered: text.trim().isNotEmpty,
        value: text,
      );
    });
  }

  int get _answeredCount => _items.where((item) => item.answered).length;

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
                'Check-in Atual',
                style: TextStyle(
                  fontSize: 18,
                  fontWeight: FontWeight.w700,
                  color: AppColors.navy,
                ),
              ),
              Text(
                '$_answeredCount of ${_items.length} answered today',
                style: const TextStyle(
                  fontSize: 12,
                  color: AppColors.textSubtle,
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),
          for (var i = 0; i < _items.length; i++) ...[
            _CheckInFormRow(
              item: _items[i],
              controller: _controllers[i],
              onChanged: (text) => _onChanged(i, text),
            ),
            if (i != _items.length - 1) const SizedBox(height: 10),
          ],
          const SizedBox(height: 20),
          SizedBox(
            width: double.infinity,
            child: FilledButton(
              onPressed: () {
                final summary = widget.initialSummary.copyWith(
                  categories: _items,
                );
                // TODO(rust): enviar `summary` para o backend aqui dentro,
                // ou deixar a cargo de quem ouve `onSubmit`.
                widget.onSubmit?.call(summary);
              },
              style: FilledButton.styleFrom(
                backgroundColor: AppColors.navy,
                padding: const EdgeInsets.symmetric(vertical: 14),
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(10),
                ),
              ),
              child: const Text('Salvar check-in'),
            ),
          ),
        ],
      ),
    );
  }
}

class _CheckInFormRow extends StatelessWidget {
  const _CheckInFormRow({
    required this.item,
    required this.controller,
    required this.onChanged,
  });

  final CheckInCategoryStatus item;
  final TextEditingController controller;
  final ValueChanged<String> onChanged;

  static const _icons = {
    CheckInCategory.sleep: Icons.bedtime_outlined,
    CheckInCategory.mood: Icons.sentiment_satisfied_alt_outlined,
    CheckInCategory.stress: Icons.monitor_heart_outlined,
    CheckInCategory.pain: Icons.bolt_outlined,
    CheckInCategory.hydration: Icons.water_drop_outlined,
    CheckInCategory.exercise: Icons.fitness_center_outlined,
    CheckInCategory.symptoms: Icons.thermostat_outlined,
  };

  static const _hints = {
    CheckInCategory.sleep: 'Ex.: dormi 7h, sono leve',
    CheckInCategory.mood: 'Ex.: bem, cansado, ansioso',
    CheckInCategory.stress: 'Ex.: baixo, médio, alto',
    CheckInCategory.pain: 'Ex.: nenhuma, leve, forte',
    CheckInCategory.hydration: 'Ex.: 1.5L até agora',
    CheckInCategory.exercise: 'Ex.: 30 min de bicicleta',
    CheckInCategory.symptoms: 'Ex.: nenhum, dor de cabeça',
  };

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: AppColors.background,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          CircleAvatar(
            radius: 18,
            backgroundColor: AppColors.border,
            child: Icon(_icons[item.category], size: 18, color: AppColors.navy),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text(
                      item.label,
                      style: const TextStyle(
                        fontSize: 14,
                        fontWeight: FontWeight.w600,
                        color: AppColors.navy,
                      ),
                    ),
                    Text(
                      item.answered ? 'Answered' : 'Not yet answered',
                      style: TextStyle(
                        fontSize: 11,
                        fontWeight: FontWeight.w600,
                        color: item.answered
                            ? AppColors.success
                            : AppColors.textSubtle,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 6),
                TextField(
                  controller: controller,
                  onChanged: onChanged,
                  style: const TextStyle(fontSize: 13),
                  decoration: InputDecoration(
                    isDense: true,
                    hintText: _hints[item.category],
                    hintStyle: const TextStyle(
                      fontSize: 12,
                      color: AppColors.textSubtle,
                    ),
                    filled: true,
                    fillColor: AppColors.surface,
                    contentPadding: const EdgeInsets.symmetric(
                      horizontal: 10,
                      vertical: 8,
                    ),
                    border: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(8),
                      borderSide: const BorderSide(color: AppColors.border),
                    ),
                    enabledBorder: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(8),
                      borderSide: const BorderSide(color: AppColors.border),
                    ),
                    focusedBorder: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(8),
                      borderSide: const BorderSide(color: AppColors.navy),
                    ),
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
