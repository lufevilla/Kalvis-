import 'entrada.dart';
import 'historico.dart';

const _escalas = {'mood_score', 'stress_score', 'anxiety_score', 'concentration_score',
  'loneliness_score', 'confinement_discomfort'};
const _vitais = ['heart_rate', 'spo2', 'blood_pressure'];

class Analise {
  final String astronauta, status;
  final List<Map<String, dynamic>> alteracoes;
  final List<String> ausentes;
  final Map<String, dynamic> contexto;
  Analise(this.astronauta, this.status, this.alteracoes, this.ausentes, this.contexto);
}

Analise analisarAlteracao(Entrada e) {
  final alts = <Map<String, dynamic>>[];
  e.valores.forEach((k, v) {
    final vals = buscarHistorico(e.astronauta, k).map((r) => r['valor']!.toDouble()).toList();
    if (vals.length < 3) return; // sem baseline confiável
    final base = vals.take(3).reduce((a, b) => a + b) / 3;
    final escala = _escalas.contains(k);
    final limiar = escala ? 2.0 : base.abs() * 0.15;
    bool fora(num x) => (x - base).abs() >= limiar;
    final brusca = (v - vals.last).abs() >= (escala ? 3 : vals.last.abs() * 0.2);
    if (!fora(v) && !brusca) return;
    alts.add({
      'indicator': k, 'atual': v, 'baseline': double.parse(base.toStringAsFixed(1)),
      'variacao': double.parse((v - base).toStringAsFixed(1)),
      'persistente': vals.reversed.take(2).every(fora),
      'brusca': brusca,
      'tendencia': vals.last > vals.first ? 'subindo' : vals.last < vals.first ? 'descendo' : 'estável',
    });
  });
  // O escalonamento do backend tem prioridade; a LLM nunca decide o status.
  final status = e.escalonamento ??
      (_vitais.any(e.ausentes.contains) ? 'insufficient_data' : alts.isEmpty ? 'normal' : 'attention');
  return Analise(e.astronauta, status, alts, e.ausentes, e.contexto);
}
