import 'dart:convert';

import 'alteracao.dart';
import 'documentos.dart';
import 'llm.dart';

const _sis = '''IA de apoio em saúde para astronautas.
Use só as observações e evidências recebidas. Escreva uma mensagem ao astronauta,
em português, com no máximo 3 frases: o que mudou, a ação recomendada (só se estiver nas evidências)
e "Isso não é diagnóstico." Responda em JSON.''';

final _schema = <String, dynamic>{
  'type': 'object',
  'properties': {'mensagem': {'type': 'string'}},
  'required': ['mensagem'],
};

bool _valido(Map<String, dynamic> o) =>
    o['mensagem'] is String && (o['mensagem'] as String).trim().isNotEmpty;

final _cache = <String, Map<String, dynamic>>{};

String _mensagem(List<Map> obs) => obs.isEmpty
    ? 'Nenhuma alteração relevante em relação ao seu histórico.'
    : 'Alterações em relação ao seu histórico: '
        '${obs.map((o) => '${o['indicator']} (${(o['finding'] as String).toLowerCase()})').join('; ')}. '
        'Isso não é diagnóstico.';

Future<Map<String, dynamic>> explicar(Llm llm, Analise a) async {
  // RAG: no máximo 3 evidências, trechos cortados em 400 caracteres.
  final evid = <String, Map<String, dynamic>>{};
  for (final alt in a.alteracoes) {
    if (evid.length >= 3) break;
    final r = consultarDocumento(
        {'type': alt['indicator'], 'context': '${a.contexto}'}, limite: 1);
    for (final e in (r['evidencias'] as List).cast<Map<String, dynamic>>()) {
      if (evid.length >= 3) break;
      evid.putIfAbsent(e['document_id'] as String, () => {
            for (final k in e.keys)
              k: e[k] is String && (e[k] as String).length > 400
                  ? (e[k] as String).substring(0, 400)
                  : e[k]
          });
    }
  }

  final obs = [
    for (final x in a.alteracoes)
      {
        'indicator': x['indicator'],
        'finding': '${(x['variacao'] as num) > 0 ? 'Acima' : 'Abaixo'} do baseline'
            '${x['persistente'] == true ? ', persistente' : ''}'
            '${x['brusca'] == true ? ', mudança brusca' : ''}',
        'evidence': 'Atual: ${x['atual']} | baseline: ${x['baseline']} | tendência: ${x['tendencia']}',
      }
  ];

  // Sem alteração: não chama a LLM. Se a LLM falhar: usa o fallback.
  var msg = _mensagem(obs);
  if (a.alteracoes.isNotEmpty) {
    final chave = jsonEncode({'s': a.status, 'o': obs, 'aus': a.ausentes});
    try {
      final o = _cache[chave] ??= await llm.json(_sis, {
        'status': a.status,
        'observacoes': obs,
        if (a.ausentes.isNotEmpty) 'ausentes': a.ausentes,
        'contexto': a.contexto,
        'evidencias': evid.values.toList(),
      }, _schema, _valido, maxTokens: 120);
      msg = o['mensagem'] as String;
    } catch (_) {}
  }

  return {
    'mensagem': msg,
    'rag_sources': [
      for (final e in evid.values) {'document_id': e['document_id'], 'page': e['page']}
    ],
  };
}