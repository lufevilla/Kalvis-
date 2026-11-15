// Recebe o alerta e o direciona para o RAG. Se não for nada, descarta.
// Caso contrário, classifica como emergência ou amarelo.
import 'dart:async';
import 'dart:convert';
import 'dart:math';

import 'package:shared_preferences/shared_preferences.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

enum Classificacao { nada, amarelo, emergencia }

enum EstadoConexao { conectando, online, offline }

// ---------------- Dados vindos do historical analysis ----------------

class Tendencia {
  Tendencia(this.direcao, this.inclinacaoH);
  final String direcao; // increasing | decreasing | stable...
  final double? inclinacaoH;

  factory Tendencia.fromJson(Map<String, dynamic> j) => Tendencia(
    j['direcao'] as String,
    (j['inclinacao_h'] as num?)?.toDouble(),
  );
}

class IndicadorAnalisado {
  IndicadorAnalisado({
    required this.indicador,
    required this.nivelOriginal,
    required this.nivelFinal,
    required this.direcao,
    this.zScore,
    this.persistente,
    this.tendencia,
  });

  final String indicador;
  final String nivelOriginal;
  final String nivelFinal;
  final String direcao;
  final double? zScore;
  final bool? persistente;
  final Tendencia? tendencia;

  bool get piorando =>
      (tendencia?.direcao == 'increasing' && direcao == 'acima') ||
      (tendencia?.direcao == 'decreasing' && direcao == 'abaixo');

  factory IndicadorAnalisado.fromJson(Map<String, dynamic> j) =>
      IndicadorAnalisado(
        indicador: j['indicador'] as String,
        nivelOriginal: j['nivel_original'] as String,
        nivelFinal: j['nivel_final'] as String,
        direcao: j['direcao'] as String,
        zScore: (j['z_score'] as num?)?.toDouble(),
        persistente: j['persistente'] as bool?,
        tendencia: j['tendencia'] == null
            ? null
            : Tendencia.fromJson(j['tendencia'] as Map<String, dynamic>),
      );
}

class Motivo {
  Motivo(this.codigo, this.indicador);
  final String codigo;
  final String? indicador;

  factory Motivo.fromJson(Map<String, dynamic> j) =>
      Motivo(j['codigo'] as String, j['indicador'] as String?);
}

class ExposicaoAmbiental {
  ExposicaoAmbiental(this.co2AcimaS, this.temperaturaAcimaS);
  final double co2AcimaS;
  final double temperaturaAcimaS;

  factory ExposicaoAmbiental.fromJson(Map<String, dynamic> j) =>
      ExposicaoAmbiental(
        (j['co2_acima_s'] as num).toDouble(),
        (j['temperatura_acima_s'] as num).toDouble(),
      );
}

class AlertaDetectado {
  AlertaDetectado(this.raw, {this.lido = false})
    : indicadores = (raw['indicadores'] as List? ?? [])
          .cast<Map<String, dynamic>>()
          .map(IndicadorAnalisado.fromJson)
          .toList(),
      motivos = (raw['motivos'] as List? ?? [])
          .cast<Map<String, dynamic>>()
          .map(Motivo.fromJson)
          .toList(),
      exposicaoAmbiental = raw['exposicao_ambiental'] == null
          ? null
          : ExposicaoAmbiental.fromJson(
              raw['exposicao_ambiental'] as Map<String, dynamic>,
            );

  final Map<String, dynamic> raw;
  final List<IndicadorAnalisado> indicadores;
  final List<Motivo> motivos;
  final ExposicaoAmbiental? exposicaoAmbiental;
  bool lido;

  String get id => raw['id'] as String;
  int get seq => (raw['seq'] as num).toInt();
  int get versao => (raw['versao'] as num?)?.toInt() ?? 1;
  String get astronautaId => raw['astronauta_id'] as String;
  String get missaoId => raw['missao_id'] as String;
  DateTime get momento => DateTime.parse(raw['momento'] as String);
  String get fase => raw['fase'] as String;
  int get diasNaFase => (raw['dias_na_fase'] as num).toInt();
  String get episodio => raw['episodio'] as String;
  String get nivelFinal => raw['nivel_final'] as String;

  String? get mensagemIa => raw['mensagem_ia'] as String?;

  bool temMotivo(String codigo) => motivos.any((m) => m.codigo == codigo);

  late final Classificacao classificacao = classificar(this);

  Map<String, dynamic> toJson() => {'raw': raw, 'lido': lido};

  factory AlertaDetectado.fromJson(Map<String, dynamic> j) => AlertaDetectado(
    j['raw'] as Map<String, dynamic>,
    lido: j['lido'] == true,
  );

  /// Converte o alerta analisado para o formato de entrada esperado pelo RAG (modeloEntrada.json).
  /// Como o alerta contém dados analisados (não telemetria bruta), inferimos valores aproximados.
  Map<String, dynamic> toRagInput() {
    // Mapear indicadores para valores fisiológicos/psicológicos aproximados
    final Map<String, double> indicadorValores = {};
    for (final ind in indicadores) {
      // Converter nível final e z-score para um valor numérico aproximado
      double baseValue = _getBaseValueForIndicator(ind.indicador);
      if (ind.zScore != null) {
        // Ajustar baseado no z-score (desvio padrão)
        indicadorValores[ind.indicador] =
            baseValue + (ind.zScore! * _getStdDevForIndicator(ind.indicador));
      } else {
        indicadorValores[ind.indicador] = baseValue;
      }
    }

    // Valores padrão baseados no nível de alerta
    final bool isEmergencia = classificacao == Classificacao.emergencia;
    final bool isAmarelo = classificacao == Classificacao.amarelo;

    // Inferir valores fisiológicos
    final double heartRate =
        indicadorValores['FC'] ??
        (isEmergencia
            ? 110
            : isAmarelo
            ? 95
            : 75);
    final double spo2 =
        indicadorValores['SpO2'] ??
        (isEmergencia
            ? 90
            : isAmarelo
            ? 94
            : 98);
    final double temp =
        indicadorValores['TEMP'] ??
        (isEmergencia
            ? 38.5
            : isAmarelo
            ? 37.8
            : 37.0);
    final double sleepHours =
        indicadorValores['SONO'] ?? (temMotivo('deficit_sono') ? 4.0 : 7.0);

    // Pressão arterial estimada
    final String bp = _estimateBloodPressure(
      indicadorValores['PAS'],
      indicadorValores['PAD'],
    );

    // Inferir valores psicológicos baseados nos motivos
    final Map<String, int> psychScores = _inferPsychScores();

    // Ambiente
    final double envTemp = exposicaoAmbiental?.temperaturaAcimaS != null
        ? 22.0 +
              (exposicaoAmbiental!.temperaturaAcimaS /
                  3600.0) // converter segundos para estimativa de graus
        : 24.0;
    final double co2Level = exposicaoAmbiental?.co2AcimaS != null
        ? 0.08 +
              (exposicaoAmbiental!.co2AcimaS / 3600000.0) // estimativa
        : 0.10;

    // Atividades
    final bool recentExercise = !temMotivo('exercicio_insuficiente');
    final bool eva =
        fase.toLowerCase().contains('eva') ||
        episodio.toLowerCase().contains('eva');

    return {
      'mission': {'mission_day': diasNaFase, 'communication_status': 'nominal'},
      'astronaut': {'id': astronautaId},
      'timestamp': momento.toIso8601String(),
      'physiological': {
        'heart_rate': heartRate.round(),
        'blood_pressure': bp,
        'spo2': spo2.round(),
        'body_temperature': double.parse(temp.toStringAsFixed(1)),
        'sleep_hours': double.parse(sleepHours.toStringAsFixed(1)),
      },
      'psychological': {
        'mood_score': psychScores['mood'] ?? 5,
        'stress_score': psychScores['stress'] ?? 5,
        'anxiety_score': psychScores['anxiety'] ?? 5,
        'concentration_score': psychScores['concentration'] ?? 5,
        'loneliness_score': psychScores['loneliness'] ?? 5,
        'confinement_discomfort': psychScores['confinement'] ?? 5,
      },
      'environment': {
        'temperature': double.parse(envTemp.toStringAsFixed(1)),
        'co2_level': double.parse(co2Level.toStringAsFixed(2)),
      },
      'activities': {
        'recent_exercise': recentExercise,
        'extravehicular_activity': eva,
      },
    };
  }

  double _getBaseValueForIndicator(String indicador) {
    switch (indicador) {
      case 'FC':
        return 75.0;
      case 'SpO2':
        return 98.0;
      case 'TEMP':
        return 37.0;
      case 'SONO':
        return 8.0;
      case 'PAS':
        return 120.0;
      case 'PAD':
        return 80.0;
      default:
        return 0.0;
    }
  }

  double _getStdDevForIndicator(String indicador) {
    switch (indicador) {
      case 'FC':
        return 10.0;
      case 'SpO2':
        return 2.0;
      case 'TEMP':
        return 0.5;
      case 'SONO':
        return 1.5;
      case 'PAS':
        return 15.0;
      case 'PAD':
        return 10.0;
      default:
        return 1.0;
    }
  }

  String _estimateBloodPressure(double? pas, double? pad) {
    final sys = (pas ?? 120).round();
    final dia = (pad ?? 80).round();
    return '$sys/$dia';
  }

  Map<String, int> _inferPsychScores() {
    final scores = <String, int>{};

    // Base scores
    scores['mood'] = 5;
    scores['stress'] = 5;
    scores['anxiety'] = 5;
    scores['concentration'] = 5;
    scores['loneliness'] = 5;
    scores['confinement'] = 5;

    // Ajustar baseado nos motivos
    if (temMotivo('episodio_ansiedade')) {
      scores['anxiety'] = 8;
      scores['stress'] = 7;
    }
    if (temMotivo('deficit_sono')) {
      scores['concentration'] = 3;
      scores['mood'] = 4;
    }
    if (temMotivo('exercicio_insuficiente')) {
      scores['mood'] = (scores['mood'] ?? 5) - 1;
    }
    if (temMotivo('co2_modulo') || temMotivo('cabine_quente')) {
      scores['confinement'] = 7;
      scores['stress'] = (scores['stress'] ?? 5) + 1;
    }
    if (temMotivo('alteracao_persistente') || temMotivo('piora_progressiva')) {
      scores['anxiety'] = (scores['anxiety'] ?? 5) + 1;
      scores['stress'] = (scores['stress'] ?? 5) + 1;
    }
    if (classificacao == Classificacao.emergencia) {
      scores['stress'] = 9;
      scores['anxiety'] = 8;
      scores['mood'] = 2;
    } else if (classificacao == Classificacao.amarelo) {
      scores['stress'] = 7;
      scores['anxiety'] = 6;
      scores['mood'] = 4;
    }

    // Clampar entre 1-10
    scores.updateAll((k, v) => v.clamp(1, 10));
    return scores;
  }
}

// ---------------- Classificação ----------------

const _motivosEmergencia = {'risco_queda_ortostatica', 'possivel_infeccao'};

const _motivosAmarelo = {
  'deficit_sono',
  'exercicio_insuficiente',
  'dado_nao_confiavel', // verificar sensor
  'episodio_ansiedade',
  'co2_modulo',
  'cabine_quente',
  'alteracao_persistente',
  'piora_progressiva',
};

Classificacao classificar(AlertaDetectado a) {
  final criticoPiorando = a.indicadores.any(
    (i) => i.nivelFinal == 'critico' && i.piorando,
  );

  if (a.nivelFinal == 'critico' ||
      criticoPiorando ||
      a.motivos.any((m) => _motivosEmergencia.contains(m.codigo))) {
    return Classificacao.emergencia;
  }
  if (a.nivelFinal == 'atencao' ||
      a.motivos.any((m) => _motivosAmarelo.contains(m.codigo))) {
    return Classificacao.amarelo;
  }
  return Classificacao.nada;
}

// ---------------- Detecter ----------------

typedef EnviarRag =
    Future<Map<String, dynamic>> Function(
      Map<String, dynamic> alerta,
      String classificacao,
    );

class Detecter {
  Detecter({required this.url, this.rag, this.astronautaId, this.limite = 500});

  final Uri url;
  final EnviarRag? rag;
  final String? astronautaId;
  final int limite;

  static const _kHistorico = 'detecter.historico';
  static const _kUltimoSeq = 'detecter.ultimo_seq';

  final _novos = StreamController<AlertaDetectado>.broadcast();
  final _respostasRag = StreamController<AlertaDetectado>.broadcast();
  final _lista = StreamController<List<AlertaDetectado>>.broadcast();
  final _estado = StreamController<EstadoConexao>.broadcast();

  late SharedPreferences _prefs;
  final List<AlertaDetectado> _historico = [];
  final Set<String> _ids = {};
  int _ultimoSeq = 0;

  WebSocketChannel? _canal;
  StreamSubscription? _sub;
  Timer? _retry;
  int _tentativas = 0;
  bool _ativo = false;

  /// Apenas amarelo/emergência.
  Stream<AlertaDetectado> get alertas => _novos.stream;
  Stream<AlertaDetectado> get respostasRag => _respostasRag.stream;
  Stream<AlertaDetectado> get emergencias =>
      alertas.where((a) => a.classificacao == Classificacao.emergencia);
  Stream<AlertaDetectado> get amarelos =>
      alertas.where((a) => a.classificacao == Classificacao.amarelo);

  Stream<List<AlertaDetectado>> get historico => _lista.stream;
  Stream<EstadoConexao> get estado => _estado.stream;
  List<AlertaDetectado> get historicoAtual => List.unmodifiable(_historico);
  int get naoLidos => _historico.where((a) => !a.lido).length;

  Future<void> iniciar() async {
    if (_ativo) return;
    _ativo = true;
    _prefs = await SharedPreferences.getInstance();
    _carregar();
    _conectar();
  }

  void _carregar() {
    _ultimoSeq = _prefs.getInt(_kUltimoSeq) ?? 0;
    final salvo = _prefs.getString(_kHistorico);
    if (salvo != null) {
      for (final j
          in (jsonDecode(salvo) as List).cast<Map<String, dynamic>>()) {
        final a = AlertaDetectado.fromJson(j);
        if (_ids.add(a.id)) _historico.add(a);
      }
    }
    _emitirLista();
  }

  Future<void> _salvar() async {
    await _prefs.setString(
      _kHistorico,
      jsonEncode(_historico.map((a) => a.toJson()).toList()),
    );
    await _prefs.setInt(_kUltimoSeq, _ultimoSeq);
  }

  void _conectar() {
    if (!_ativo) return;
    _estado.add(EstadoConexao.conectando);
    final uri = url.replace(
      queryParameters: {
        ...url.queryParameters,
        'desde': '$_ultimoSeq',
        'astronauta_id': ?astronautaId,
      },
    );
    try {
      _canal = WebSocketChannel.connect(uri);
      _canal!.ready
          .then((_) {
            _tentativas = 0;
            _estado.add(EstadoConexao.online);
          })
          .catchError((_) => _reagendar());
      _sub = _canal!.stream.listen(
        _mensagem,
        onError: (_) => _reagendar(),
        onDone: _reagendar,
        cancelOnError: true,
      );
    } catch (_) {
      _reagendar();
    }
  }

  void _reagendar() {
    _sub?.cancel();
    _sub = null;
    _canal = null;
    if (!_ativo || _retry?.isActive == true) return;
    _estado.add(EstadoConexao.offline);
    final s = min(30, pow(2, _tentativas++).toInt());
    _retry = Timer(Duration(seconds: s), _conectar);
  }

  Future<void> _mensagem(dynamic msg) async {
    final Map<String, dynamic> j;
    try {
      j = jsonDecode(msg as String) as Map<String, dynamic>;
    } catch (_) {
      return;
    }
    switch (j['tipo']) {
      case 'analise.identificada':
        final AlertaDetectado a;
        try {
          a = AlertaDetectado(j);
        } catch (_) {
          return; // payload inválido
        }
        _ultimoSeq = max(_ultimoSeq, a.seq);

        // Sem alteração relevante: descarta.
        if (a.classificacao == Classificacao.nada || !_ids.add(a.id)) {
          await _prefs.setInt(_kUltimoSeq, _ultimoSeq);
          return;
        }

        _historico.insert(0, a);
        _historico.sort((x, y) => y.seq.compareTo(x.seq));
        while (_historico.length > limite) {
          _ids.remove(_historico.removeLast().id);
        }
        await _salvar();
        _novos.add(a);
        _emitirLista();
        unawaited(_enviarParaRag(a));
      case 'stream.ressincronizar':
        await _canal?.sink.close(); // reconecta a partir de _ultimoSeq
    }
  }

  Future<void> _enviarParaRag(AlertaDetectado a) async {
    if (rag == null) return;
    try {
      final ragInput = a.toRagInput();
      final res = await rag!(ragInput, a.classificacao.name);
      a.raw['mensagem_ia'] = res['mensagem'];
      a.raw['fontes_ia'] = res['rag_sources'];
    } catch (e) {
      a.raw['mensagem_ia'] =
          'Não foi possível gerar a explicação. Procure o oficial médico.';
      a.raw['erro_ia'] = '$e';
    }
    if (!_ativo) return;
    await _salvar();
    _respostasRag.add(a);
    _emitirLista();
  }

  void _emitirLista() => _lista.add(List.unmodifiable(_historico));

  Future<void> marcarLido(String id) async {
    for (final a in _historico) {
      if (a.id == id) a.lido = true;
    }
    await _salvar();
    _emitirLista();
  }

  Future<void> marcarTodosLidos() async {
    for (final a in _historico) {
      a.lido = true;
    }
    await _salvar();
    _emitirLista();
  }

  Future<void> limpar() async {
    _historico.clear();
    _ids.clear();
    await _salvar();
    _emitirLista();
  }

  Future<void> encerrar() async {
    _ativo = false;
    _retry?.cancel();
    await _sub?.cancel();
    await _canal?.sink.close();
    await _novos.close();
    await _respostasRag.close();
    await _lista.close();
    await _estado.close();
  }
}
