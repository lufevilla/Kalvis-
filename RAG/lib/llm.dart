import 'dart:convert';
import 'dart:io';

final ollamaUrl = () {
  final u = Platform.environment['OLLAMA_URL'] ?? 'http://localhost:11434';
  if (!['localhost', '127.0.0.1'].contains(Uri.parse(u).host)) {
    throw StateError('Apenas Ollama local (offline)');
  }
  return u;
}();

final modelo = Platform.environment['OLLAMA_MODEL'] ?? 'qwen2.5:3b';

class Metricas {
  int chamadas = 0, entrada = 0, saida = 0, jsonTotal = 0, jsonInvalido = 0;
  double segGeracao = 0, segTotal = 0;

  num _n(Map r, String k) => (r[k] ?? 0) as num;

  void registrar(Map r) {
    chamadas++;
    entrada += _n(r, 'prompt_eval_count').toInt();
    saida += _n(r, 'eval_count').toInt();
    segGeracao += _n(r, 'eval_duration') / 1e9;
    segTotal += _n(r, 'total_duration') / 1e9;
  }

  Map<String, dynamic> resumo() => {
        'chamadas': chamadas,
        'tokens_entrada': entrada,
        'tokens_saida': saida,
        'tokens_por_s': segGeracao > 0 ? (saida / segGeracao).toStringAsFixed(1) : '-',
        'latencia_media_s': chamadas > 0 ? (segTotal / chamadas).toStringAsFixed(2) : '-',
        'json_valido': jsonTotal > 0
            ? '${(100 * (jsonTotal - jsonInvalido) / jsonTotal).round()}%'
            : '-',
      };
}

class Llm {
  final _http = HttpClient()..connectionTimeout = const Duration(seconds: 20);
  final metricas = Metricas();

  Future<Map<String, dynamic>> chat(List<Map<String, dynamic>> msgs,
      {Object? format, List? tools, double temp = 0.1, int maxTokens = 300}) async {
    final req = await _http.postUrl(Uri.parse('$ollamaUrl/api/chat'));
    req.headers.contentType = ContentType.json;
    req.write(jsonEncode({
      'model': modelo,
      'stream': false,
      'think': false,
      'keep_alive': '30m',
      'messages': msgs,
      if (format != null) 'format': format,
      if (tools != null) 'tools': tools,
      'options': {
        'num_ctx': 2048,
        'num_thread': 4,
        'temperature': temp,
        'num_predict': maxTokens,
      },
    }));
    final res = await req.close().timeout(const Duration(seconds: 180));
    final body = await utf8.decodeStream(res);
    if (res.statusCode != 200) throw HttpException('Ollama ${res.statusCode}: $body');
    final r = jsonDecode(body) as Map<String, dynamic>;
    metricas.registrar(r);
    return Map<String, dynamic>.from(r['message'] as Map);
  }

  /// Saída JSON validada, com 1 nova tentativa.
  Future<Map<String, dynamic>> json(String sistema, Object dados,
      Map<String, dynamic> schema, bool Function(Map<String, dynamic>) valido,
      {int maxTokens = 350}) async {
    for (var t = 0; t < 2; t++) {
      metricas.jsonTotal++;
      final m = await chat([
        {'role': 'system', 'content': sistema},
        {'role': 'user', 'content': jsonEncode(dados)},
      ], format: schema, temp: 0, maxTokens: maxTokens);
      try {
        final o = jsonDecode(m['content'] as String);
        if (o is Map<String, dynamic> && valido(o)) return o;
      } catch (_) {}
      metricas.jsonInvalido++;
    }
    throw const FormatException('LLM retornou JSON inválido após nova tentativa');
  }

  void fechar() => _http.close(force: true);
}
