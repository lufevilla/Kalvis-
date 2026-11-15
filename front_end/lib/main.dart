import 'package:flutter/material.dart';
import 'package:front_end/core/app_colors.dart';
import 'package:front_end/core/alert_detect/detecter.dart';
import 'package:front_end/core/navigation.dart';
import 'package:front_end/mocks/menu_mocks.dart';
import 'package:front_end/rag.dart';
import 'package:front_end/screens/home/home_screen.dart';
import 'package:front_end/screens/tracking/tracking_screen.dart';
import 'package:front_end/widgets/diagnosis_listener.dart';
import 'package:front_end/widgets/layout/app_shell.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized(); // necessário p/ SharedPreferences
  runApp(const KalvisApp());
}

class KalvisApp extends StatefulWidget {
  const KalvisApp({super.key});

  @override
  State<KalvisApp> createState() => _KalvisAppState();
}

class _KalvisAppState extends State<KalvisApp> {
  late final Detecter _detecter = Detecter(
    url: Uri.parse('ws://10.0.2.2:8080/ws/analises'),
    rag: RagService().analisarAlerta,
  );

  @override
  void initState() {
    super.initState();
    _detecter.iniciar();
  }

  @override
  void dispose() {
    _detecter.encerrar();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Kalvis',
      navigatorKey: navigatorKey,
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        useMaterial3: true,
        colorScheme: ColorScheme.fromSeed(seedColor: AppColors.navy),
        scaffoldBackgroundColor: AppColors.background,
      ),
      home: DiagnosisListener(
        detecter: _detecter,
        child: AppShell(
          dateLabel: todayLabel,
          pages: {
            'Home': const HomeScreen(),
            'Tracking': const TrackingScreen(),
          },
        ),
      ),
    );
  }
}