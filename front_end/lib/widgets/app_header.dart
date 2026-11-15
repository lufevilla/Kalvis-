import 'package:flutter/material.dart';

import '../../core/app_colors.dart';

class AppHeader extends StatefulWidget {
  const AppHeader({super.key, required this.title, required this.dateLabel});

  final String title;
  final String dateLabel;

  @override
  State<AppHeader> createState() => _AppHeaderState();
}

class _AppHeaderState extends State<AppHeader> {
  

  @override
  void dispose() {
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 72,
      padding: const EdgeInsets.symmetric(horizontal: 24),
      decoration: const BoxDecoration(
        color: AppColors.surface,
        border: Border(bottom: BorderSide(color: AppColors.border)),
      ),
      child: Row(
        children: [
          Column(
            mainAxisAlignment: MainAxisAlignment.center,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                widget.title,
                style: const TextStyle(
                  fontSize: 20,
                  fontWeight: FontWeight.w700,
                  color: AppColors.navy,
                ),
              ),
              Text(
                widget.dateLabel,
                style: const TextStyle(
                  fontSize: 13,
                  color: AppColors.textSubtle,
                ),
              ),
            ],
          ),
          const Spacer(),
        ],
      ),
    );
  }
}
