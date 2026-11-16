import 'package:flutter/material.dart';
import 'package:front_end/core/alert_detect/detecter.dart';
import 'package:front_end/core/app_colors.dart';
import 'package:front_end/core/models/tracking.dart';
import 'package:front_end/core/services/tracking_database_service.dart';
import 'package:front_end/widgets/responsive_grid.dart';
import 'package:front_end/widgets/tracking_metric_card.dart';

/// Conteúdo da tela Tracking. Sidebar e header vêm do AppShell.
/// Dados vêm do banco SQLite via TrackingDatabaseService.
class TrackingScreen extends StatefulWidget {
  const TrackingScreen({super.key, this.alerta});

  final AlertaDetectado? alerta;

  @override
  State<TrackingScreen> createState() => _TrackingScreenState();
}

class _TrackingScreenState extends State<TrackingScreen> {
  final _dbService = TrackingDatabaseService();
  bool _isLoading = true;
  String? _error;
  TrackingData? _data;

  @override
  void initState() {
    super.initState();
    _initializeAndLoad();
  }

  Future<void> _initializeAndLoad() async {
    setState(() {
      _isLoading = true;
      _error = null;
    });
    try {
      await TrackingDatabaseService().initialize();
      TrackingDatabaseService().dataStream.listen((data) {
        if (mounted) {
          setState(() {
            _data = data;
          });
        }
      });
      final data = TrackingDatabaseService().currentData;
      if (data != null) {
        setState(() => _data = data);
      }
    } catch (e) {
      setState(() => _error = 'Erro ao inicializar: $e');
    } finally {
      if (mounted) setState(() => _isLoading = false);
    }
  }

  @override
  void dispose() {
    TrackingDatabaseService().dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return RefreshIndicator(
      onRefresh: () async {
        final data = await TrackingDatabaseService().fetchTrackingData();
        if (data != null && mounted) {
          setState(() => _data = data);
        }
      },
      child: SingleChildScrollView(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.all(24),
        child: _isLoading
            ? const Center(child: CircularProgressIndicator())
            : _error != null
                ? Column(
                    children: [
                      const Icon(Icons.error_outline, size: 48, color: Colors.red),
                      const SizedBox(height: 16),
                      Text(_error!, style: const TextStyle(color: Colors.red)),
                      const SizedBox(height: 16),
                      ElevatedButton(
                        onPressed: _initializeAndLoad,
                        child: const Text('Tentar novamente'),
                      ),
                    ],
                  )
                : _data == null
                    ? const Center(child: Text('Nenhum dado disponível'))
                    : Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          if (widget.alerta != null) ...[
                            _SectionTitle('Sintomas/Alertas (IA)'),
                            const SizedBox(height: 8),
                            Card(
                              color: widget.alerta!.classificacao == Classificacao.emergencia
                                  ? Colors.red.shade50
                                  : Colors.orange.shade50,
                              child: Padding(
                                padding: const EdgeInsets.all(16),
                                child: Column(
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  children: [
                                    Text(
                                      'Episódio: ${widget.alerta!.episodio} • Nível: ${widget.alerta!.nivelFinal}',
                                      style: const TextStyle(fontWeight: FontWeight.w700, color: AppColors.navy),
                                    ),
                                    if (widget.alerta!.mensagemIa != null && widget.alerta!.mensagemIa!.isNotEmpty) ...[
                                      const SizedBox(height: 8),
                                      Text(widget.alerta!.mensagemIa!, style: const TextStyle(color: AppColors.textMuted)),
                                    ],
                                    const SizedBox(height: 8),
                                    Text(
                                      'Motivos: ${widget.alerta!.motivos.map((m) => m.codigo).join(', ')}',
                                      style: const TextStyle(fontSize: 12, color: AppColors.textSubtle),
                                    ),
                                  ],
                                ),
                              ),
                            ),
                            const SizedBox(height: 24),
                          ],
                          _SectionTitle('Monitoramento Fisiológico (Crew Vitals)'),
                          const SizedBox(height: 16),
                          ResponsiveGrid(
                            children: [
                              for (final vital in _data!.crewVitals)
                                TrackingMetricCard(metric: vital.toTrackingMetric()),
                            ],
                          ),
                          const SizedBox(height: 32),
                          Row(
                            mainAxisAlignment: MainAxisAlignment.spaceBetween,
                            children: [
                              _SectionTitle('Ambiente & Espaço (Módulo Orbital)'),
                              if (_data!.currentModule != null)
                                Chip(
                                  label: Text(_data!.currentModule!),
                                  backgroundColor: AppColors.navy.withOpacity(0.1),
                                ),
                            ],
                          ),
                          const SizedBox(height: 16),
                          ResponsiveGrid(
                            children: [
                              for (final env in _data!.environmentMetrics)
                                TrackingMetricCard(metric: env.toTrackingMetric()),
                            ],
                          ),
                        ],
                      ),
      ),
    );
  }
}

class _SectionTitle extends StatelessWidget {
  const _SectionTitle(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    return Text(
      text,
      style: const TextStyle(
        fontSize: 20,
        fontWeight: FontWeight.w700,
        color: AppColors.navy,
      ),
    );
  }
}