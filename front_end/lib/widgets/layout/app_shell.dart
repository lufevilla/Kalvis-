import 'package:flutter/material.dart';
import 'package:front_end/core/app_colors.dart';
import 'package:front_end/widgets/app_header.dart';
import 'package:front_end/widgets/app_sidebar.dart';

/// Layout fixo presente em todas as telas.
/// Sidebar e header ficam sempre visíveis; só o `child` muda entre rotas.
class AppShell extends StatelessWidget {
  const AppShell({
    super.key,
    required this.title,
    required this.dateLabel,
    required this.child,
  });

  final String title;
  final String dateLabel;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: AppColors.background,
      body: Row(
        children: [
          const AppSidebar(),
          Expanded(
            child: Column(
              children: [
                AppHeader(title: title, dateLabel: dateLabel),
                Expanded(child: child),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
