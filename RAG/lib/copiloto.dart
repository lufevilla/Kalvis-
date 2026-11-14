import 'dart:convert';
import 'documentos.dart';
import 'historico.dart';
import 'llm.dart';

const _sis = '''Você é uma inteligência artificial de apoio em saúde.
Responda em até 4 frases simples. Use as ferramentas para histórico ou documentos.
Sem evidência, diga que não encontrou.

REGRAS OBRIGATÓRIAS:
- Causalidade: nunca afirme causa. Diga "não é possível afirmar que X causou Y; os dados mostram apenas associação".
- Emoções: não simule amizade nem prolongue a conversa. Sempre diga "sou uma inteligência artificial" e recomende conversar com a tripulação e com o oficial médico da missão.''';

final _tools = [
  {'type': 'function', 'function': {'name': 'consultar_documento',
    'description': 'Busca evidência médica',
    'parameters': {'type': 'object', 'properties': {
      'type': {'type': 'string', 'description': 'indicador, ex: heart_rate'},
      'context': {'type': 'string', 'description': 'palavras-chave curtas'}},
      'required': ['type', 'context']}}},
  {'type': 'function', 'function': {'name': 'buscar_historico',
    'description': 'Registros anteriores de um indicador',
    'parameters': {'type': 'object', 'properties': {
      'indicator': {'type': 'string'}, 'dias': {'type': 'integer'}},
      'required': ['indicator']}}},
];

final _reEmocional = RegExp(r'sozinh|solid|triste|amig|ansios|deprimid|saudade|medo');
final _reCausal = RegExp(r'caus(ou|a|ando)|culpa|por causa|provoc');

class Copiloto {
  final Llm llm;
  final String astronauta;
  final _msgs = <Map<String, dynamic>>[];
  var _resumo = '';
  Copiloto(this.llm, this.astronauta);

  Future<String> perguntar(String texto) async {
    final p = texto.trim();
    if (p.isEmpty || p.length > 1000) {
      throw const FormatException('Mensagem vazia ou acima de 1000 caracteres');
    }
    _msgs.add({'role': 'user', 'content': p});
    await _compactar();

    for (var i = 0; i < 3; i++) {
      final m = await llm.chat([
        {'role': 'system', 'content': _resumo.isEmpty ? _sis : '$_sis\nResumo anterior: $_resumo'},
        ..._msgs,
      ], tools: _tools, temp: 0.2, maxTokens: 200);

      final calls = m['tool_calls'] as List?;
      if (calls == null || calls.isEmpty) {
        var r = '${m['content'] ?? ''}'.trim();
        if (r.isEmpty) r = 'Não encontrei informação suficiente.';
        r = _garantirSeguranca(p, r);
        _msgs.add({'role': 'assistant', 'content': r});
        return r;
      }

      _msgs.add(m);
      for (final c in calls) {
        final f = c['function'] as Map;
        Map<String, dynamic> args;
        try {
          args = f['arguments'] is String
              ? jsonDecode(f['arguments']) as Map<String, dynamic>
              : Map<String, dynamic>.from(f['arguments'] as Map);
        } catch (_) {
          args = {};
        }
        _msgs.add({
          'role': 'tool',
          'tool_name': f['name'],
          'content': jsonEncode(_executar('${f['name']}', args)),
        });
      }
    }
    return _garantirSeguranca(p, 'Não consegui concluir a consulta. Procure o oficial médico da missão.');
  }

  String _garantirSeguranca(String pergunta, String resp) {
    final q = pergunta.toLowerCase();
    if (_reEmocional.hasMatch(q)) {
      if (!resp.toLowerCase().contains('inteligência artificial')) {
        resp = 'Sou uma inteligência artificial e não substituo o contato humano. $resp';
      }
      if (!RegExp(r'tripula|oficial médico').hasMatch(resp.toLowerCase())) {
        resp += ' Recomendo conversar com a tripulação e com o oficial médico da missão.';
      }
    }
    if (_reCausal.hasMatch(q) && !resp.toLowerCase().contains('não é possível afirmar')) {
      resp += ' Não é possível afirmar causalidade; os dados mostram apenas associação.';
    }
    return resp;
  }

  Object _executar(String nome, Map<String, dynamic> a) => switch (nome) {
        'consultar_documento' => consultarDocumento(a),
        'buscar_historico' => {
            'registros': buscarHistorico(astronauta, '${a['indicator']}',
                dias: (a['dias'] as num?)?.toInt() ?? 7)
          },
        _ => {'erro': 'ferramenta desconhecida'},
      };

  Future<void> _compactar() async {
    final tam = _msgs.fold<int>(0, (s, m) => s + '${m['content']}'.length);
    if (_msgs.length < 12 && tam < 6000) return;
    final r = await llm.chat([
      {'role': 'system', 'content': 'Resuma em até 60 palavras os fatos de saúde e pedidos do usuário. Sem opinião.'},
      {'role': 'user', 'content': jsonEncode([
        if (_resumo.isNotEmpty) _resumo,
        ..._msgs.take(_msgs.length - 1).map((m) => '${m['role']}: ${m['content']}'),
      ])},
    ], temp: 0, maxTokens: 120);
    _resumo = '${r['content']}'.trim();
    _msgs.removeRange(0, _msgs.length - 1);
  }
}
