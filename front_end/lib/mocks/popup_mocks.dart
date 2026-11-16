import 'package:flutter/material.dart';

/// ---------- Notificações ----------

class NotificationData {
  const NotificationData({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.time,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final String time;
}

/// MOCK — lista de notificações.
/// TODO: substituir por notificações reais, ordenadas por data/hora.
const mockNotifications = <NotificationData>[
  NotificationData(
    icon: Icons.lock_outline,
    title: 'Acesso de dados',
    subtitle: 'Acesso de sistema',
    time: '01:03:59',
  ),
  NotificationData(
    icon: Icons.warning_amber_rounded,
    title: 'Alerta de rede',
    subtitle: 'Alerta de rede detectado',
    time: '01:03:59',
  ),
  NotificationData(
    icon: Icons.info_outline,
    title: 'Sistema OK',
    subtitle: 'Sistema operando normalmente',
    time: '01:03:59',
  ),
];

/// ---------- Alerta de tripulante em risco ----------

class CrewRiskAlertData {
  const CrewRiskAlertData({
    required this.crewName,
    required this.riskReason,
    required this.shipLocation,
  });

  final String crewName;
  final String riskReason;
  final String shipLocation;
}

/// MOCK — alerta de risco de um tripulante.
/// TODO: alimentar com o alerta real quando disparado (ex.: evento de monitoramento).
const mockCrewRiskAlert = CrewRiskAlertData(
  crewName: 'Nome do Tripulante',
  riskReason: 'Motivo do risco detectado pela IA',
  shipLocation: 'Espaçonave #03',
);

/// ---------- Check-in atual ----------

enum CheckInItemBadgeType { none, inProgress, label }

class CurrentCheckInItemData {
  const CurrentCheckInItemData({
    required this.icon,
    required this.label,
    required this.statusText,
    this.badgeType = CheckInItemBadgeType.none,
  });

  final IconData icon;
  final String label;
  final String statusText;
  final CheckInItemBadgeType badgeType;
}

/// MOCK — status do check-in em andamento no momento.
/// TODO: substituir pelo progresso real do check-in do tripulante.
const mockCurrentCheckInItems = <CurrentCheckInItemData>[
  CurrentCheckInItemData(
    icon: Icons.fitness_center_outlined,
    label: 'Exercício em andamento',
    statusText: 'EM ANDAMENTO',
    badgeType: CheckInItemBadgeType.inProgress,
  ),
  CurrentCheckInItemData(
    icon: Icons.sentiment_satisfied_alt_outlined,
    label: 'Mood',
    statusText: 'Não respondido',
  ),
  CurrentCheckInItemData(
    icon: Icons.bolt_outlined,
    label: 'Pain',
    statusText: 'Não respondido',
  ),
  CurrentCheckInItemData(
    icon: Icons.water_drop_outlined,
    label: 'Hydration',
    statusText: 'Não respondido',
  ),
];

/// ---------- Diagnóstico da IA ----------

class DiagnosisFindingData {
  const DiagnosisFindingData({
    required this.icon,
    required this.title,
    required this.subtitle,
    this.isAlert = false,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final bool isAlert;
}

/// MOCK — achados do diagnóstico gerado pela IA.
/// TODO: substituir pelo diagnóstico real gerado a partir dos dados do tripulante.
const mockDiagnosisFindings = <DiagnosisFindingData>[
  DiagnosisFindingData(
    icon: Icons.favorite_border,
    title: 'BPM acima do normal',
    subtitle: 'Relaxe, converse com o médico da tripulação e se acalme',
    isAlert: true,
  ),
  DiagnosisFindingData(
    icon: Icons.water_drop_outlined,
    title: 'Hidratação baixa',
    subtitle: 'Hidratação abaixo do recomendado, busque tomar mais água',
  ),
];

/// MOCK — recomendações textuais da IA.
/// TODO: substituir pelas recomendações reais geradas pela IA.
const mockDiagnosisRecommendations = <String>[
  'Ouça a recomendação completa da IA.',
  'Priorize as informações mais recentes da espaçonave.',
  'Compartilhe os sintomas com a equipe médica.',
  'Confirme as atividades recomendadas no check-in.',
];
