import 'package:flutter/material.dart';

/// Grid responsivo: 4 colunas em telas largas, 2 em médias, 1 em estreitas.
/// Cards da mesma linha ficam com a mesma altura.
class ResponsiveGrid extends StatelessWidget {
  const ResponsiveGrid({super.key, required this.children, this.spacing = 16});

  final List<Widget> children;
  final double spacing;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final columns = constraints.maxWidth >= 1000
            ? 4
            : constraints.maxWidth >= 600
            ? 2
            : 1;

        final rows = <List<Widget>>[];
        for (var i = 0; i < children.length; i += columns) {
          final end = (i + columns) > children.length
              ? children.length
              : i + columns;
          rows.add(children.sublist(i, end));
        }

        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            for (var r = 0; r < rows.length; r++) ...[
              if (r > 0) SizedBox(height: spacing),
              IntrinsicHeight(
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    for (var c = 0; c < rows[r].length; c++) ...[
                      if (c > 0) SizedBox(width: spacing),
                      Expanded(child: rows[r][c]),
                    ],
                  ],
                ),
              ),
            ],
          ],
        );
      },
    );
  }
}
