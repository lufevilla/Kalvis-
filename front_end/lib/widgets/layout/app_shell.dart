import 'package:flutter/material.dart';

import '../../core/app_colors.dart';
import '../../core/alert_detect/detecter.dart';
import 'package:front_end/screens/tracking/tracking_screen.dart';
import 'package:front_end/widgets/app_header.dart';
import 'package:front_end/widgets/app_sidebar.dart';

/// Layout fixo com sidebar + header. Mantém o item ativo e troca o conteúdo
/// conforme o clique na sidebar. Itens sem tela mostram "Em breve".
class AppShell extends StatefulWidget {
  const AppShell({
    super.key,
    required this.pages,
    required this.dateLabel,
    this.initialLabel = 'Home',
    this.detecter,
  });

  /// Tela de cada item, pela label. Ex.: {'Home': HomeScreen(...)}
  final Map<String, Widget> pages;
  final String dateLabel;
  final String initialLabel;
  final dynamic detecter;

  @override
  State<AppShell> createState() => _AppShellState();
}

class _AppShellState extends State<AppShell> {
  late String _activeLabel;
  AlertaDetectado? _ultimoAlerta;

  @override
  void initState() {
    super.initState();
    _activeLabel = widget.initialLabel;
    if (widget.detecter != null) {
      final d = widget.detecter as Detecter;
      d.alertas.listen((a) {
        if (mounted) setState(() => _ultimoAlerta = a);
      });
      d.respostasRag.listen((a) {
        if (mounted) setState(() => _ultimoAlerta = a);
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    Widget? page = widget.pages[_activeLabel];
    if (_activeLabel == 'Tracking') {
      page = _ultimoAlerta != null ? TrackingScreen(alerta: _ultimoAlerta) : const TrackingScreen();
    }
    page ??= const _ComingSoon();

    return Scaffold(
      backgroundColor: AppColors.background,
      body: Row(
        children: [
          AppSidebar(
            activeLabel: _activeLabel,
            onItemSelected: (label) => setState(() => _activeLabel = label),
          ),
          Expanded(
            child: Column(
              children: [
                AppHeader(title: _activeLabel, dateLabel: widget.dateLabel),
                Expanded(child: page),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _ComingSoon extends StatelessWidget {
  const _ComingSoon();

  @override
  Widget build(BuildContext context) {
    return const Center(
      child: Text(
        'Em breve',
        style: TextStyle(fontSize: 16, color: AppColors.textSubtle),
      ),
    );
  }
}
