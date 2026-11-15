import 'package:flutter/material.dart';
import 'package:front_end/core/models/checkin.dart';

/// Como a pergunta é respondida: chips (seleção rápida) ou número (ex.: minutos).
enum CheckInFieldType { chips, number }

/// Uma pergunta do pop-up de check-in.
///
/// `options` só é usado quando `type == chips`. `multi` permite marcar
/// mais de uma opção (ex.: Symptoms). `unitLabel` só é usado quando
/// `type == number` (ex.: "minutos").
class CheckInQuestion {
  const CheckInQuestion({
    required this.category,
    required this.label,
    required this.icon,
    required this.type,
    this.options = const [],
    this.multi = false,
    this.unitLabel,
  });

  final CheckInCategory category;
  final String label;
  final IconData icon;
  final CheckInFieldType type;
  final List<String> options;
  final bool multi;
  final String? unitLabel;
}

/// Resposta de uma pergunta: 0, 1 ou várias opções selecionadas
/// (número vira uma lista com 1 item, ex.: ["30"]).
class CheckInAnswer {
  const CheckInAnswer({required this.category, this.selected = const []});

  final CheckInCategory category;
  final List<String> selected;

  bool get isAnswered => selected.isNotEmpty;

  CheckInAnswer copyWith({List<String>? selected}) {
    return CheckInAnswer(
      category: category,
      selected: selected ?? this.selected,
    );
  }
}
