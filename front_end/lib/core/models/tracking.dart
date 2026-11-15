import 'package:flutter/material.dart';

/// Estado visual de uma métrica.
enum TrackingStatus { normal, attention, alert }

/// Dados da barra de faixa normal (ex.: BPM entre 60 e 100).
class RangeBarData {
  const RangeBarData({
    required this.value,
    required this.scaleMin,
    required this.scaleMax,
    required this.normalMin,
    required this.normalMax,
    required this.rangeLabel,
  });

  final double value;
  final double scaleMin;
  final double scaleMax;
  final double normalMin;
  final double normalMax;

  /// Texto exibido abaixo da barra, ex.: "Normal range  60–100 bpm".
  final String rangeLabel;
}

/// Dados vitais da tripulação (do backend).
class TrackingVital {
  final String title;
  final String value;
  final String unit;
  final String status;
  final String statusLabel;
  final double? rangeMin;
  final double? rangeMax;
  final double? normalMin;
  final double? normalMax;
  final String? description;
  final String? actionText;
  final IconData icon;

  TrackingVital({
    required this.title,
    required this.value,
    required this.unit,
    required this.status,
    required this.statusLabel,
    this.rangeMin,
    this.rangeMax,
    this.normalMin,
    this.normalMax,
    this.description,
    this.actionText,
    required this.icon,
  });

  factory TrackingVital.fromJson(Map<String, dynamic> json) {
    return TrackingVital(
      title: json['title'] as String,
      value: json['value'] as String,
      unit: json['unit'] as String,
      status: json['status'] as String,
      statusLabel: json['status_label'] as String,
      rangeMin: (json['range_min'] as num?)?.toDouble(),
      rangeMax: (json['range_max'] as num?)?.toDouble(),
      normalMin: (json['normal_min'] as num?)?.toDouble(),
      normalMax: (json['normal_max'] as num?)?.toDouble(),
      description: json['description'] as String?,
      actionText: json['action_text'] as String?,
      icon: _iconForTitle(json['title'] as String),
    );
  }

  factory TrackingVital.fromTrackingMetric(TrackingMetric metric) {
    return TrackingVital(
      title: metric.title,
      value: metric.value,
      unit: metric.unit,
      status: _statusToString(metric.status),
      statusLabel: metric.statusLabel,
      rangeMin: metric.rangeBar?.scaleMin,
      rangeMax: metric.rangeBar?.scaleMax,
      normalMin: metric.rangeBar?.normalMin,
      normalMax: metric.rangeBar?.normalMax,
      description: metric.description,
      actionText: metric.actionText,
      icon: metric.icon,
    );
  }

  TrackingMetric toTrackingMetric() {
    return TrackingMetric(
      icon: icon,
      title: title,
      value: value,
      unit: unit,
      status: _parseStatus(status),
      statusLabel: statusLabel,
      rangeBar: rangeMin != null
          ? RangeBarData(
              value: double.tryParse(value) ?? 0,
              scaleMin: rangeMin ?? 0,
              scaleMax: rangeMax ?? 100,
              normalMin: normalMin ?? 0,
              normalMax: normalMax ?? 100,
              rangeLabel: normalMin != null && normalMax != null
                  ? 'Normal range   $normalMin–$normalMax $unit'
                  : '',
            )
          : null,
      description: description,
      actionText: actionText,
    );
  }

  static IconData _iconForTitle(String title) {
    switch (title) {
      case 'Frequência Cardíaca':
        return Icons.monitor_heart_outlined;
      case 'Oxigenação (SpO2)':
        return Icons.air;
      case 'Temperatura Corporal':
        return Icons.thermostat;
      case 'Nível de Fadiga / Estresse':
        return Icons.psychology_outlined;
      default:
        return Icons.help_outline;
    }
  }

  static String _statusToString(TrackingStatus status) {
    switch (status) {
      case TrackingStatus.normal:
        return 'normal';
      case TrackingStatus.attention:
        return 'attention';
      case TrackingStatus.alert:
        return 'alert';
    }
  }

  TrackingStatus _parseStatus(String status) {
    switch (status) {
      case 'normal':
        return TrackingStatus.normal;
      case 'attention':
        return TrackingStatus.attention;
      case 'alert':
        return TrackingStatus.alert;
      default:
        return TrackingStatus.normal;
    }
  }
}

/// Dados ambientais (do backend).
class TrackingEnvironment {
  final String title;
  final String value;
  final String unit;
  final String status;
  final String statusLabel;
  final String? description;
  final String? actionText;
  final IconData icon;

  TrackingEnvironment({
    required this.title,
    required this.value,
    required this.unit,
    required this.status,
    required this.statusLabel,
    this.description,
    this.actionText,
    required this.icon,
  });

  factory TrackingEnvironment.fromJson(Map<String, dynamic> json) {
    return TrackingEnvironment(
      title: json['title'] as String,
      value: json['value'] as String,
      unit: json['unit'] as String,
      status: json['status'] as String,
      statusLabel: json['status_label'] as String,
      description: json['description'] as String?,
      actionText: json['action_text'] as String?,
      icon: _iconForTitle(json['title'] as String),
    );
  }

  factory TrackingEnvironment.fromTrackingMetric(TrackingMetric metric) {
    return TrackingEnvironment(
      title: metric.title,
      value: metric.value,
      unit: metric.unit,
      status: _statusToString(metric.status),
      statusLabel: metric.statusLabel,
      description: metric.description,
      actionText: metric.actionText,
      icon: metric.icon,
    );
  }

  TrackingMetric toTrackingMetric() {
    return TrackingMetric(
      icon: icon,
      title: title,
      value: value,
      unit: unit,
      status: _parseStatus(status),
      statusLabel: statusLabel,
      description: description,
      actionText: actionText,
    );
  }

  static IconData _iconForTitle(String title) {
    switch (title) {
      case 'Radiação Cósmica':
        return Icons.blur_on;
      case 'Pressão da Cabine':
        return Icons.speed_outlined;
      case 'Nível de CO2':
        return Icons.cloud_outlined;
      case 'Oxigênio da Cabine (O2)':
        return Icons.science_outlined;
      default:
        return Icons.help_outline;
    }
  }

  static String _statusToString(TrackingStatus status) {
    switch (status) {
      case TrackingStatus.normal:
        return 'normal';
      case TrackingStatus.attention:
        return 'attention';
      case TrackingStatus.alert:
        return 'alert';
    }
  }

  TrackingStatus _parseStatus(String status) {
    switch (status) {
      case 'normal':
        return TrackingStatus.normal;
      case 'attention':
        return TrackingStatus.attention;
      case 'alert':
        return TrackingStatus.alert;
      default:
        return TrackingStatus.normal;
    }
  }
}

/// Resposta completa de dados de tracking.
class TrackingData {
  final List<TrackingVital> crewVitals;
  final List<TrackingEnvironment> environmentMetrics;
  final String lastUpdate;
  final String astronautId;
  final String? missionName;
  final String? currentModule;

  TrackingData({
    required this.crewVitals,
    required this.environmentMetrics,
    required this.lastUpdate,
    required this.astronautId,
    this.missionName,
    this.currentModule,
  });

  factory TrackingData.fromJson(Map<String, dynamic> json) {
    return TrackingData(
      crewVitals: (json['crew_vitals'] as List)
          .map((v) => TrackingVital.fromJson(v))
          .toList(),
      environmentMetrics: (json['environment_metrics'] as List)
          .map((e) => TrackingEnvironment.fromJson(e))
          .toList(),
      lastUpdate: json['last_update'] as String,
      astronautId: json['astronaut_id'] as String,
      missionName: json['mission_name'] as String?,
      currentModule: json['current_module'] as String?,
    );
  }
}

/// Uma métrica exibida em um card da tela de Tracking.
class TrackingMetric {
  const TrackingMetric({
    required this.icon,
    required this.title,
    required this.value,
    required this.status,
    required this.statusLabel,
    this.unit = '',
    this.description,
    this.rangeBar,
    this.waveform,
    this.waveformFilled = false,
    this.actionText,
  });

  final IconData icon;
  final String title;
  final String value;
  final String unit;
  final TrackingStatus status;

  /// Texto do selo (ex.: "Normal", "Atenção", "Alerta: Alta").
  final String statusLabel;

  /// Texto de apoio abaixo do valor, opcional.
  final String? description;

  /// Barra de faixa normal, opcional.
  final RangeBarData? rangeBar;

  /// Onda/sparkline com valores entre 0 (baixo) e 1 (topo), opcional.
  final List<double>? waveform;
  final bool waveformFilled;

  /// Ação recomendada no rodapé, opcional.
  final String? actionText;

  factory TrackingMetric.fromVital(TrackingVital vital) => TrackingMetric(
    icon: vital.icon,
    title: vital.title,
    value: vital.value,
    unit: vital.unit,
    status: vital._parseStatus(vital.status),
    statusLabel: vital.statusLabel,
    rangeBar: vital.rangeMin != null
        ? RangeBarData(
            value: double.tryParse(vital.value) ?? 0,
            scaleMin: vital.rangeMin ?? 0,
            scaleMax: vital.rangeMax ?? 100,
            normalMin: vital.normalMin ?? 0,
            normalMax: vital.normalMax ?? 100,
            rangeLabel: vital.normalMin != null && vital.normalMax != null
                ? 'Normal range   ${vital.normalMin}–${vital.normalMax} ${vital.unit}'
                : '',
          )
        : null,
    description: vital.description,
    actionText: vital.actionText,
  );

  factory TrackingMetric.fromEnvironment(TrackingEnvironment env) =>
      TrackingMetric(
        icon: env.icon,
        title: env.title,
        value: env.value,
        unit: env.unit,
        status: env._parseStatus(env.status),
        statusLabel: env.statusLabel,
        description: env.description,
        actionText: env.actionText,
      );
}
