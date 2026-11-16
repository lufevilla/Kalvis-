// lib/widgets/diagnosis_listener.dart
import 'dart:async';
import 'package:flutter/material.dart';
import 'package:front_end/core/navigation.dart';
import 'package:front_end/widgets/popups/ai_diagnosis_card.dart';
import 'package:front_end/core/alert_detect/detecter.dart';
import 'package:front_end/core/services/diagnosis_mapper.dart';

class DiagnosisListener extends StatefulWidget {
  const DiagnosisListener({
    super.key,
    required this.detecter,
    required this.child,
  });
  final Detecter detecter;
  final Widget child;

  @override
  State<DiagnosisListener> createState() => _DiagnosisListenerState();
}

class _DiagnosisListenerState extends State<DiagnosisListener> {
  StreamSubscription? _sub;
  bool _aberto = false;

  @override
  void initState() {
    super.initState();
    _sub = widget.detecter.respostasRag.listen(_mostrar);
  }

  Future<void> _mostrar(AlertaDetectado a) async {
    final ctx = navigatorKey.currentContext;
    if (ctx == null || _aberto) return; // evita empilhar pop-ups
    _aberto = true;
    final v = mapearDiagnostico(a);

    await showDialog(
      context: ctx,
      barrierDismissible: a.classificacao != Classificacao.emergencia,
      builder: (dCtx) => Dialog(
        backgroundColor: Colors.transparent,
        insetPadding: const EdgeInsets.all(20),
        child: SingleChildScrollView(
          child: AiDiagnosisCard(
            findings: v.findings,
            recommendations: v.recommendations,
            aiMessage: (a.raw['mensagem_ia'] as String?) ?? v.textoFala,
            onListen: () {
              /* tts.speak(v.textoFala) */
            },
            onGoToChat: () {
              Navigator.pop(dCtx);
              navigatorKey.currentState?.pushNamed('/chat', arguments: a);
            },
          ),
        ),
      ),
    );
    _aberto = false;
    await widget.detecter.marcarLido(a.id);
  }

  @override
  void dispose() {
    _sub?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
