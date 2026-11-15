import 'package:flutter/material.dart';

import '../../core/app_colors.dart';
import '../../core/models/tracking.dart';

/// Barra horizontal: faixa cinza, zona normal em verde e marcador do valor.
///
/// Usa Row + Expanded com `flex` em vez de LayoutBuilder, porque
/// LayoutBuilder não suporta dimensões intrínsecas (IntrinsicHeight).
class RangeBar extends StatelessWidget {
  const RangeBar({super.key, required this.data});

  final RangeBarData data;

  /// Resolução da escala em flex (0..1000).
  static const _scale = 1000;

  double _fraction(double value) {
    final f = (value - data.scaleMin) / (data.scaleMax - data.scaleMin);
    return f < 0 ? 0 : (f > 1 ? 1 : f);
  }

  /// Converte um valor para flex entre 1 e _scale - 1 (flex precisa ser > 0).
  int _flex(double value) {
    final flex = (_fraction(value) * _scale).round();
    if (flex < 1) return 1;
    if (flex > _scale - 1) return _scale - 1;
    return flex;
  }

  @override
  Widget build(BuildContext context) {
    final normalStart = _flex(data.normalMin);
    final normalEnd = _flex(data.normalMax);
    final normalWidth = (normalEnd - normalStart) < 1
        ? 1
        : normalEnd - normalStart;
    final markerAt = _flex(data.value);

    return SizedBox(
      height: 14,
      child: Stack(
        clipBehavior: Clip.none,
        children: [
          // Faixa: cinza | verde (normal) | cinza
          Positioned.fill(
            top: 4,
            bottom: 4,
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Expanded(flex: normalStart, child: _segment(AppColors.border)),
                Expanded(flex: normalWidth, child: _segment(AppColors.success)),
                Expanded(
                  flex: _scale - normalEnd,
                  child: _segment(AppColors.border),
                ),
              ],
            ),
          ),
          // Marcador do valor atual
          Positioned.fill(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Expanded(flex: markerAt, child: const SizedBox()),
                Container(
                  width: 3,
                  decoration: BoxDecoration(
                    color: AppColors.navy,
                    borderRadius: BorderRadius.circular(2),
                  ),
                ),
                Expanded(flex: _scale - markerAt, child: const SizedBox()),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _segment(Color color) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: color,
        borderRadius: BorderRadius.circular(4),
      ),
    );
  }
}
