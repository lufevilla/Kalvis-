import 'package:flutter/material.dart';
import 'package:front_end/screens/home/home_screen.dart';

import 'core/app_colors.dart';
import 'mocks/menu_mocks.dart';
import 'widgets/layout/app_shell.dart';

void main() {
  runApp(const KalvisApp());
}

class KalvisApp extends StatelessWidget {
  const KalvisApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Kalvis',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        useMaterial3: true,
        colorScheme: ColorScheme.fromSeed(seedColor: AppColors.navy),
        scaffoldBackgroundColor: AppColors.background,
      ),
      home: AppShell(
        title: 'Home',
        dateLabel: mockTodayLabel,
        child: HomeScreen(),
      ),
    );
  }
}
