// lib/services/diagnosis_mapper.dart
import 'package:flutter/material.dart';
import 'package:front_end/mocks/popup_mocks.dart'; // DiagnosisFindingData
import 'package:front_end/core/alert_detect/detecter.dart';

const _nomes = {
  'heart_rate': 'Frequência cardíaca',
  'spo2': 'Saturação de O₂',
  'systolic': 'Pressão sistólica',
  'diastolic': 'Pressão diastólica',
  'sleep_hours': 'Sono',
  'stress_score': 'Estresse',
  'anxiety_score': 'Ansiedade',
  'mood_score': 'Humor',
  'loneliness_score': 'Solidão',
  'co2_level': 'CO₂',
  'temperature': 'Temperatura',
};

const _acoes = {
  'risco_queda_ortostatica':
      'Levante-se devagar e evite esforço até nova avaliação.',
  'possivel_infeccao': 'Contate o oficial médico da missão imediatamente.',
  'deficit_sono': 'Priorize o próximo período de descanso.',
  'exercicio_insuficiente': 'Retome a rotina de exercícios prevista.',
  'dado_nao_confiavel': 'Verifique o posicionamento do sensor.',
  'episodio_ansiedade': 'Faça uma pausa e converse com a tripulação.',
  'co2_modulo': 'Verifique a ventilação do módulo.',
  'cabine_quente': 'Ajuste o controle térmico da cabine.',
};

IconData _icone(String ind) => switch (ind) {
  'heart_rate' || 'systolic' || 'diastolic' => Icons.favorite_outline,
  'spo2' => Icons.air,
  'sleep_hours' => Icons.bedtime_outlined,
  'co2_level' || 'temperature' => Icons.thermostat_outlined,
  _ => Icons.psychology_outlined,
};

class DiagnosisView {
  DiagnosisView(this.findings, this.recommendations, this.textoFala);
  final List<DiagnosisFindingData> findings;
  final List<String> recommendations;
  final String textoFala;
}

DiagnosisView mapearDiagnostico(AlertaDetectado a) {
  final findings = [
    for (final i in a.indicadores.where((i) => i.nivelFinal != 'normal'))
      DiagnosisFindingData(
        icon: _icone(i.indicador),
        title: _nomes[i.indicador] ?? i.indicador,
        subtitle:
            '${i.direcao == 'acima' ? 'Acima' : 'Abaixo'} do seu histórico'
            '${i.persistente == true ? ' • persistente' : ''}'
            '${i.piorando ? ' • piorando' : ''}',
        isAlert: i.nivelFinal == 'critico',
      ),
  ];

  final msg = a.mensagemIa ?? 'Analisando...';
  final recs = <String>{
    msg, // texto da IA
    for (final m in a.motivos)
      if (_acoes[m.codigo] != null) _acoes[m.codigo]!,
  }.toList();

  return DiagnosisView(findings, recs, msg);
}
