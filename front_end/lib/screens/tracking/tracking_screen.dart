import 'package:flutter/material.dart';
import 'package:front_end/core/app_colors.dart';
import 'package:front_end/core/models/tracking.dart';
import 'package:front_end/core/services/tracking_api_service.dart' show TrackingApiService;
import 'package:front_end/mocks/tracking_mocks.dart';
import 'package:front_end/widgets/responsive_grid.dart';
import 'package:front_end/widgets/tracking_metric_card.dart';
import 'package:path_provider/path_provider.dart';

/// Conteúdo da tela Tracking. Sidebar e header vêm do AppShell.
/// Dados vêm do backend via TrackingApiService.
class TrackingScreen extends StatefulWidget {
  const TrackingScreen({super.key});

  @override
  State<TrackingScreen> createState() => _TrackingScreenState();
}

class _TrackingScreenState extends State<TrackingScreen> {
  final _api = TrackingApiService();
  bool _isLoading = true;
  String? _error;
  TrackingData? _data;

  @override
  void initState() {
    super.initState();
    _loadData();
  }

  Future<void> _loadData() async {
    setState(() {
      _isLoading = true;
      _error = null;
    });
    try {
      _data = await _api.fetchTrackingData();
    } catch (e) {
      _error = 'Erro ao carregar dados: $e';
      // Fallback to mocks on error
      _data = TrackingData(
        crewVitals: mockCrewVitals
            .map((m) => TrackingVital.fromTrackingMetric(m))
            .toList(),
        environmentMetrics: mockEnvironmentMetrics
            .map((m) => TrackingEnvironment.fromTrackingMetric(m))
            .toList(),
        lastUpdate: DateTime.now().toIso8601String(),
        astronautId: 'AST001',
      );
    } finally {
      if (mounted) setState(() => _isLoading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return RefreshIndicator(
      onRefresh: _loadData,
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
                        onPressed: _loadData,
                        child: const Text('Tentar novamente'),
                      ),
                    ],
                  )
                : Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
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