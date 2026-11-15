import 'package:flutter/material.dart';
import 'package:front_end/mocks/chekin_popup_mocks.dart';
import 'package:front_end/widgets/popups/checkin_popup.dart';

import '../../core/app_colors.dart';

/// Pop-up de check-in diário: perguntas por chips (seleção rápida) e um
/// campo numérico para exercício (minutos).
///
/// Uso:
///   showDialog(
///     context: context,
///     builder: (_) => CheckInPopupDialog(
///       onSubmit: (answers) {
///         // TODO(rust): enviar `answers` para o backend
///       },
///     ),
///   );
class CheckInPopupDialog extends StatefulWidget {
  const CheckInPopupDialog({
    super.key,
    this.questions = mockCheckInQuestions,
    this.onSubmit,
  });

  /// MOCK por padrão — passe as perguntas reais quando vierem do backend.
  final List<CheckInQuestion> questions;

  /// TODO(rust): enviar as respostas para o backend, ex.: POST /checkins/today
  final ValueChanged<List<CheckInAnswer>>? onSubmit;

  @override
  State<CheckInPopupDialog> createState() => _CheckInPopupDialogState();
}

class _CheckInPopupDialogState extends State<CheckInPopupDialog> {
  late List<CheckInAnswer> _answers;

  @override
  void initState() {
    super.initState();
    _answers = widget.questions
        .map((q) => CheckInAnswer(category: q.category))
        .toList();
  }

  int get _answeredCount => _answers.where((a) => a.isAnswered).length;

  void _setAnswer(int index, List<String> selected) {
    setState(
      () => _answers[index] = _answers[index].copyWith(selected: selected),
    );
  }

  void _toggleChip(int index, String option, bool multi) {
    final current = _answers[index].selected;
    List<String> next;
    if (multi) {
      next = List.of(current);
      if (next.contains(option)) {
        next.remove(option);
      } else {
        next.add(option);
      }
    } else {
      next = current.contains(option) ? [] : [option];
    }
    _setAnswer(index, next);
  }

  @override
  Widget build(BuildContext context) {
    final total = widget.questions.length;

    return Dialog(
      backgroundColor: Colors.transparent,
      insetPadding: const EdgeInsets.all(24),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 460, maxHeight: 640),
        child: Container(
          decoration: BoxDecoration(
            color: AppColors.surface,
            borderRadius: BorderRadius.circular(20),
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Padding(
                padding: const EdgeInsets.fromLTRB(24, 20, 24, 0),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text(
                      'Daily check-in',
                      style: TextStyle(
                        fontSize: 18,
                        fontWeight: FontWeight.w700,
                        color: AppColors.navy,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      '$_answeredCount of $total answered',
                      style: const TextStyle(
                        fontSize: 12,
                        color: AppColors.textSubtle,
                      ),
                    ),
                  ],
                ),
              ),
              Padding(
                padding: const EdgeInsets.fromLTRB(24, 14, 24, 0),
                child: ClipRRect(
                  borderRadius: BorderRadius.circular(2),
                  child: LinearProgressIndicator(
                    value: total == 0 ? 0 : _answeredCount / total,
                    minHeight: 4,
                    backgroundColor: AppColors.border,
                    valueColor: const AlwaysStoppedAnimation(AppColors.success),
                  ),
                ),
              ),
              Flexible(
                child: SingleChildScrollView(
                  padding: const EdgeInsets.fromLTRB(24, 16, 24, 16),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      for (var i = 0; i < widget.questions.length; i++) ...[
                        _QuestionBlock(
                          question: widget.questions[i],
                          answer: _answers[i],
                          onChipTap: (option) =>
                              _toggleChip(i, option, widget.questions[i].multi),
                          onNumberChanged: (value) =>
                              _setAnswer(i, value.isEmpty ? [] : [value]),
                        ),
                        if (i != widget.questions.length - 1)
                          const SizedBox(height: 20),
                      ],
                    ],
                  ),
                ),
              ),
              Container(
                padding: const EdgeInsets.fromLTRB(24, 16, 24, 24),
                decoration: const BoxDecoration(
                  border: Border(top: BorderSide(color: AppColors.border)),
                ),
                child: Row(
                  children: [
                    Expanded(
                      child: OutlinedButton(
                        onPressed: () => Navigator.of(context).maybePop(),
                        style: OutlinedButton.styleFrom(
                          foregroundColor: AppColors.navy,
                          side: const BorderSide(color: AppColors.border),
                          padding: const EdgeInsets.symmetric(vertical: 14),
                          shape: RoundedRectangleBorder(
                            borderRadius: BorderRadius.circular(10),
                          ),
                        ),
                        child: const Text('Fechar'),
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: FilledButton(
                        onPressed: () {
                          widget.onSubmit?.call(_answers);
                          Navigator.of(context).pop(_answers);
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
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _QuestionBlock extends StatelessWidget {
  const _QuestionBlock({
    required this.question,
    required this.answer,
    required this.onChipTap,
    required this.onNumberChanged,
  });

  final CheckInQuestion question;
  final CheckInAnswer answer;
  final ValueChanged<String> onChipTap;
  final ValueChanged<String> onNumberChanged;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            CircleAvatar(
              radius: 14,
              backgroundColor: AppColors.background,
              child: Icon(question.icon, size: 14, color: AppColors.navy),
            ),
            const SizedBox(width: 8),
            Text(
              question.label,
              style: const TextStyle(
                fontSize: 14,
                fontWeight: FontWeight.w600,
                color: AppColors.navy,
              ),
            ),
          ],
        ),
        const SizedBox(height: 10),
        if (question.type == CheckInFieldType.number)
          _NumberField(
            unitLabel: question.unitLabel ?? '',
            initialValue: answer.selected.isEmpty ? '' : answer.selected.first,
            onChanged: onNumberChanged,
          )
        else
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              for (final option in question.options)
                _Chip(
                  label: option,
                  selected: answer.selected.contains(option),
                  onTap: () => onChipTap(option),
                ),
            ],
          ),
        if (question.multi) ...[
          const SizedBox(height: 4),
          const Text(
            'Pode selecionar mais de um',
            style: TextStyle(fontSize: 11, color: AppColors.textSubtle),
          ),
        ],
      ],
    );
  }
}

class _Chip extends StatelessWidget {
  const _Chip({
    required this.label,
    required this.selected,
    required this.onTap,
  });

  final String label;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: selected ? AppColors.navy : AppColors.surface,
      borderRadius: BorderRadius.circular(20),
      child: InkWell(
        borderRadius: BorderRadius.circular(20),
        onTap: onTap,
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(20),
            border: Border.all(
              color: selected ? AppColors.navy : AppColors.border,
            ),
          ),
          child: Text(
            label,
            style: TextStyle(
              fontSize: 13,
              color: selected ? Colors.white : AppColors.navy,
            ),
          ),
        ),
      ),
    );
  }
}

class _NumberField extends StatefulWidget {
  const _NumberField({
    required this.unitLabel,
    required this.initialValue,
    required this.onChanged,
  });

  final String unitLabel;
  final String initialValue;
  final ValueChanged<String> onChanged;

  @override
  State<_NumberField> createState() => _NumberFieldState();
}

class _NumberFieldState extends State<_NumberField> {
  late final TextEditingController _controller = TextEditingController(
    text: widget.initialValue,
  );

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        SizedBox(
          width: 90,
          child: TextField(
            controller: _controller,
            keyboardType: TextInputType.number,
            onChanged: widget.onChanged,
            style: const TextStyle(fontSize: 14),
            decoration: InputDecoration(
              isDense: true,
              hintText: '0',
              contentPadding: const EdgeInsets.symmetric(
                horizontal: 10,
                vertical: 10,
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
        ),
        const SizedBox(width: 10),
        Text(
          widget.unitLabel,
          style: const TextStyle(fontSize: 13, color: AppColors.textSubtle),
        ),
      ],
    );
  }
}
