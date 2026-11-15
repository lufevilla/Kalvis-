import 'package:flutter/material.dart';

/// Onda desenhada a partir de valores entre 0 (baixo) e 1 (topo).
/// Se `fillColor` for informado, preenche a área abaixo da linha.
class WaveformChart extends StatelessWidget {
  const WaveformChart({
    super.key,
    required this.points,
    required this.color,
    this.fillColor,
  });

  final List<double> points;
  final Color color;
  final Color? fillColor;

  @override
  Widget build(BuildContext context) {
    return CustomPaint(
      size: Size.infinite,
      painter: _WaveformPainter(
        points: points,
        color: color,
        fillColor: fillColor,
      ),
    );
  }
}

class _WaveformPainter extends CustomPainter {
  _WaveformPainter({required this.points, required this.color, this.fillColor});

  final List<double> points;
  final Color color;
  final Color? fillColor;

  @override
  void paint(Canvas canvas, Size size) {
    if (points.length < 2) return;

    final stepX = size.width / (points.length - 1);
    final line = Path();

    for (var i = 0; i < points.length; i++) {
      final x = i * stepX;
      final y = size.height * (1 - points[i]);
      if (i == 0) {
        line.moveTo(x, y);
      } else {
        line.lineTo(x, y);
      }
    }

    final fill = fillColor;
    if (fill != null) {
      final area = Path()
        ..addPath(line, Offset.zero)
        ..lineTo(size.width, size.height)
        ..lineTo(0, size.height)
        ..close();
      canvas.drawPath(
        area,
        Paint()
          ..color = fill
          ..style = PaintingStyle.fill,
      );
    }

    canvas.drawPath(
      line,
      Paint()
        ..color = color
        ..style = PaintingStyle.stroke
        ..strokeWidth = 2
        ..strokeJoin = StrokeJoin.round,
    );
  }

  @override
  bool shouldRepaint(covariant _WaveformPainter oldDelegate) {
    return oldDelegate.points != points ||
        oldDelegate.color != color ||
        oldDelegate.fillColor != fillColor;
  }
}
