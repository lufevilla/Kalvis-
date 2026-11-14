import 'dart:convert';
import 'dart:io';

/// category → indicadores do modeloEntrada que ela cobre.
const _indicadores = {
  'cardiovascular': ['heart_rate', 'systolic', 'diastolic', 'spo2'],
  'psychological': ['mood_score', 'stress_score', 'anxiety_score', 'concentration_score',
      'loneliness_score', 'confinement_discomfort'],
  'sleep': ['sleep_hours'],
  'environment': ['co2_level', 'temperature'],
  'operations': <String>[],
};

/// Sinônimos para melhorar a busca sem embeddings.
const _sinonimos = {
  'heart_rate': 'frequência cardíaca batimento bpm pulso coração',
  'systolic': 'pressão arterial', 'diastolic': 'pressão arterial',
  'sleep_hours': 'sono dormir fadiga circadiano',
  'loneliness_score': 'solidão isolamento social',
  'stress_score': 'estresse comportamental', 'anxiety_score': 'ansiedade comportamental',
  'mood_score': 'humor bem-estar', 'concentration_score': 'concentração cognição',
  'confinement_discomfort': 'confinamento isolamento',
  'co2_level': 'dióxido carbono co2 ar', 'temperature': 'temperatura ambiental',
};

final _trechos = <Map<String, dynamic>>[];

Set<String> _tok(String s) =>
    s.toLowerCase().split(RegExp(r'[^a-z0-9à-ú_]+')).where((w) => w.length > 2).toSet();

void carregarDocumentos([String pasta = 'documentos/aprovados']) {
  final dir = Directory(pasta);
  if (!dir.existsSync()) throw StateError('Pasta $pasta não encontrada');
  _trechos.clear();
  final ids = <String>{};
  for (final f in dir.listSync().whereType<File>().where((f) => f.path.endsWith('.json'))) {
    final d = jsonDecode(f.readAsStringSync());
    if (d is! Map<String, dynamic>) throw FormatException('Inválido: ${f.path}');
    for (final c in ['id', 'title', 'version', 'source', 'text', 'category']) {
      if (d[c] is! String || (d[c] as String).trim().isEmpty) {
        throw FormatException('${f.path}: campo $c inválido');
      }
    }
    if (d['approved'] != true) continue;
    if (!ids.add(d['id'])) throw StateError('ID duplicado: ${d['id']}');

    // Cada parágrafo vira um trecho; "page" = nº do parágrafo.
    final paras = (d['text'] as String).split(RegExp(r'\n\s*\n'));
    for (var i = 0; i < paras.length; i++) {
      _trechos.add({
        'document_id': d['id'], 'title': d['title'],
        'source': (d['source'] as String).split('|').first.trim(),
        'section': d['category'], 'indicators': _indicadores[d['category']] ?? [],
        'topic': d['title'], 'population': RegExp(r'^(CDC|Organização Mundial)').hasMatch(d['source'] as String)
    ? 'Adultos na Terra (aplicabilidade espacial incerta)' : 'Astronautas', 'version': d['version'],
        'page': i + 1,
        'authority': d['clinical_protocol_approved'] == true
            ? 'Protocolo clínico aprovado' : 'Contexto científico (não é protocolo)',
        'protocolo': d['clinical_protocol_approved'] == true,
        'text': paras[i].trim(),
        '_tok': _tok('${d['title']} ${paras[i]}'),
      });
    }
  }
  if (_trechos.isEmpty) throw StateError('Nenhum documento carregado');
}

/// Tool: {type, context} → {found, evidencias[]}
Map<String, dynamic> consultarDocumento(Map<String, dynamic> q, {int limite = 2}) {
  final tipo = '${q['type'] ?? ''}'.toLowerCase().trim();
  final termos = _tok('$tipo ${_sinonimos[tipo] ?? ''} ${q['context'] ?? ''}');
  final ranq = <(int, Map<String, dynamic>)>[];
  for (final t in _trechos) {
    final s = ((t['indicators'] as List).contains(tipo) ? 3 : 0) +
        termos.intersection(t['_tok'] as Set<String>).length;
    if (s >= 3) ranq.add((s, t));
  }
  if (ranq.isEmpty) return {'found': false, 'evidencias': []};
  ranq.sort((a, b) => b.$1.compareTo(a.$1));
  final vistos = <String>{};
  return {
    'found': true,
    'evidencias': [
      for (final (_, t) in ranq)
        if (vistos.add(t['document_id']) && vistos.length <= limite)
          {for (final k in ['document_id', 'section', 'page', 'authority', 'protocolo', 'population', 'text']) k: t[k]}
    ],
  };
}
