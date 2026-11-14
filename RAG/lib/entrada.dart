const _faixas = <String, List<num>>{
  'physiological.heart_rate': [20, 250],
  'physiological.spo2': [50, 100],
  'physiological.body_temperature': [30, 45],
  'physiological.sleep_hours': [0, 24],
  'psychological.mood_score': [0, 10],
  'psychological.stress_score': [0, 10],
  'psychological.anxiety_score': [0, 10],
  'psychological.concentration_score': [0, 10],
  'psychological.loneliness_score': [0, 10],
  'psychological.confinement_discomfort': [0, 10],
  'environment.temperature': [-10, 50],
  'environment.co2_level': [0, 5],
};
const escalonamentos = {'normal', 'attention', 'critical'};

class Entrada {
  final String astronauta;
  final Map<String, num> valores;
  final List<String> ausentes;
  final Map<String, dynamic> contexto;
  final String? escalonamento; // vindo do backend
  Entrada(this.astronauta, this.valores, this.ausentes, this.contexto, this.escalonamento);
}

Entrada validarEntrada(dynamic j) {
  if (j is! Map<String, dynamic>) throw const FormatException('Entrada deve ser objeto JSON');
  final erros = <String>[], ausentes = <String>[], valores = <String, num>{};

  final id = (j['astronaut'] as Map?)?['id'];
  if (id is! String || id.isEmpty) erros.add('astronaut.id ausente');
  if (DateTime.tryParse('${j['timestamp']}') == null) erros.add('timestamp inválido');

  _faixas.forEach((k, f) {
    final p = k.split('.');
    final v = (j[p[0]] as Map?)?[p[1]];
    if (v == null) return ausentes.add(p[1]);
    if (v is! num) return erros.add('$k não numérico');
    if (v < f[0] || v > f[1]) return erros.add('$k fora da faixa ${f[0]}–${f[1]}');
    valores[p[1]] = v;
  });

  final pa = (j['physiological'] as Map?)?['blood_pressure'];
  if (pa == null) {
    ausentes.add('blood_pressure');
  } else {
    final m = RegExp(r'^(\d{2,3})/(\d{2,3})$').firstMatch('$pa');
    if (m == null) {
      erros.add('blood_pressure deve ser "SIS/DIA"');
    } else {
      valores['systolic'] = int.parse(m[1]!);
      valores['diastolic'] = int.parse(m[2]!);
    }
  }

  final act = (j['activities'] as Map?) ?? {};
  for (final k in ['recent_exercise', 'extravehicular_activity']) {
    if (act[k] != null && act[k] is! bool) erros.add('activities.$k deve ser booleano');
  }
  final esc = j['backend_escalation'];
  if (esc != null && !escalonamentos.contains(esc)) erros.add('backend_escalation inválido');
  if (erros.isNotEmpty) throw FormatException(erros.join('; '));

  return Entrada(id as String, valores, ausentes, {
    'dia_missao': (j['mission'] as Map?)?['mission_day'],
    'comunicacao': (j['mission'] as Map?)?['communication_status'],
    'exercicio_recente': act['recent_exercise'],
    'eva': act['extravehicular_activity'],
    'temp_ambiente': valores['temperature'],
    'co2': valores['co2_level'],
  }, esc as String?);
}
