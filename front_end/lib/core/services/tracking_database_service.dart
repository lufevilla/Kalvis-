import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:sqflite_common_ffi/sqflite_ffi.dart';
import 'package:path/path.dart';
import 'package:front_end/core/models/checkin.dart';
import 'package:front_end/core/models/tracking.dart';
import 'package:front_end/mocks/tracking_mocks.dart';
import 'package:front_end/widgets/popups/checkin_popup.dart';

class TrackingDatabaseService {
  static final TrackingDatabaseService _instance = TrackingDatabaseService._internal();
  factory TrackingDatabaseService() => _instance;
  TrackingDatabaseService._internal();

  Database? _db;
  Timer? _refreshTimer;
  StreamController<TrackingData>? _dataController;
  bool _disposed = false;
  bool _initialized = false;

  Stream<TrackingData> get dataStream => _dataController?.stream ?? const Stream.empty();
  TrackingData? _currentData;
  TrackingData? get currentData => _currentData;
  bool get isInitialized => _initialized;

  Future<void> initialize() async {
    if (_disposed) {
      _disposed = false;
    }
    
    if (!_initialized) {
      _dataController = StreamController<TrackingData>.broadcast();
      await _openDatabase();
      await _loadInitialData();
      _startPeriodicRefresh();
      _initialized = true;
    }
  }

  Future<void> _openDatabase() async {
    String? dbPath;
    
    final possiblePaths = [
      '/home/felipe/dev/hackatons/NASA SPACE APPS/back-end/data_base/kalvis.db',
      '/home/felipe/dev/hackatons/NASA SPACE APPS/front_end/build/linux/x64/release/bundle/back-end/data_base/kalvis.db',
    ];

    for (final path in possiblePaths) {
      if (await File(path).exists()) {
        _db = await openDatabase(path);
        print('Database opened at: $path');
        return;
      }
    }

    throw Exception('Database file not found');
  }

  Future<void> _loadInitialData() async {
    final data = await _fetchTrackingData();
    _currentData = data;
    if (data != null && !_disposed && _dataController != null) {
      _dataController!.add(data);
    }
  }

  void _startPeriodicRefresh() {
    _refreshTimer?.cancel();
    _refreshTimer = Timer.periodic(const Duration(seconds: 5), (_) async {
      if (_disposed) return;
      final data = await _fetchTrackingData();
      if (data != null && !_disposed && _dataController != null) {
        _currentData = data;
        _dataController!.add(data);
      }
    });
  }

  Future<TrackingData?> fetchTrackingData() async {
    return _fetchTrackingData();
  }

  Future<TrackingData?> _fetchTrackingData() async {
    if (_db == null || _disposed) return null;

    try {
      final telemetry = await _db!.rawQuery('''
        SELECT 
          id, astronauta_id, missao_id, timestamp_utc,
          frequencia_cardiaca_bpm, spo2_percentual,
          temperatura_corporal_c, temperatura_pele_c,
          taxa_respiratoria_rpm, hrv_ms,
          pressao_sistolica_mmhg, pressao_diastolica_mmhg,
          glicose_mg_dl, origem_medicao
        FROM telemetria_fisiologica
        WHERE astronauta_id = 'AST001'
          AND deleted_at IS NULL
        ORDER BY timestamp_utc DESC
        LIMIT 1
      ''');

      if (telemetry.isEmpty) {
        return _getMockData();
      }

      final t = telemetry.first;

      final missaoId = t['missao_id'] as String?;
      List<Map<String, dynamic>> envSummaries = [];
      
      if (missaoId != null && missaoId.isNotEmpty) {
        envSummaries = await _db!.rawQuery('''
          SELECT 
            mn.id AS modulo_id,
            mn.nome AS nome_modulo,
            m.id AS missao_id,
            m.nome_missao AS nome_missao,
            COUNT(a.id) AS quantidade_amostras,
            AVG(a.co2_ppm) AS media_co2_ppm,
            MAX(a.co2_ppm) AS max_co2_ppm,
            AVG(a.radiacao_msv_hora) AS media_radiacao_msv_hora,
            MAX(a.radiacao_msv_hora) AS max_radiacao_msv_hora,
            AVG(a.temperatura_cabine_c) AS media_temperatura_cabine_c,
            MIN(a.temperatura_cabine_c) AS min_temperatura_cabine_c,
            MAX(a.temperatura_cabine_c) AS max_temperatura_cabine_c,
            AVG(a.pressao_kpa) AS media_pressao_kpa,
            MIN(a.pressao_kpa) AS min_pressao_kpa,
            MAX(a.pressao_kpa) AS max_pressao_kpa,
            MIN(a.timestamp_utc) AS primeira_amostra_utc,
            MAX(a.timestamp_utc) AS ultima_amostra_utc
          FROM modulos_nave mn
          JOIN missoes m ON m.id = mn.missao_id AND m.deleted_at IS NULL
          LEFT JOIN ambiente_nave a 
            ON a.modulo_id = mn.id
            AND a.deleted_at IS NULL
          WHERE mn.missao_id = ?
            AND mn.deleted_at IS NULL
          GROUP BY mn.id, mn.nome, m.id, m.nome_missao
          ORDER BY mn.nome
        ''', [missaoId]);
      }

      List<Map<String, dynamic>> registros = [];
      try {
        registros = await _db!.rawQuery('''
          SELECT horas_sono_total, humor_score, nivel_estresse_subjetivo, fadiga_score
          FROM registros_diarios
          WHERE astronauta_id = 'AST001' AND deleted_at IS NULL
          ORDER BY data_registro DESC LIMIT 1
        ''');
      } catch (_) {}

      return _buildTrackingData(telemetry.first, envSummaries, registros.isNotEmpty ? registros.first : null);
    } catch (e) {
      print('Error fetching tracking data: $e');
      return _getMockData();
    }
  }

  TrackingData _buildTrackingData(
    Map<String, dynamic> t,
    List<Map<String, dynamic>> envSummaries, [
    Map<String, dynamic>? registro,
  ]) {
    final crewVitals = <TrackingVital>[];

    final hr = t['frequencia_cardiaca_bpm'];
    if (hr != null) {
      final hrVal = (hr as num).toDouble();
      crewVitals.add(TrackingVital(
        title: 'Frequência Cardíaca',
        value: hrVal.round().toString(),
        unit: 'bpm',
        status: (hrVal >= 60 && hrVal <= 100) ? 'normal' : 'attention',
        statusLabel: (hrVal >= 60 && hrVal <= 100) ? 'Normal' : 'Atenção',
        rangeMin: 40,
        rangeMax: 140,
        normalMin: 60,
        normalMax: 100,
        icon: Icons.monitor_heart_outlined,
      ));
    }

    final spo2 = t['spo2_percentual'];
    if (spo2 != null) {
      final spo2Val = (spo2 as num).toDouble();
      crewVitals.add(TrackingVital(
        title: 'Oxigenação (SpO2)',
        value: spo2Val.round().toString(),
        unit: '%',
        status: spo2Val >= 95 ? 'normal' : 'alert',
        statusLabel: spo2Val >= 95 ? 'Normal' : 'Alerta',
        rangeMin: 85,
        rangeMax: 100,
        normalMin: 95,
        normalMax: 100,
        icon: Icons.air,
      ));
    }

    final temp = t['temperatura_corporal_c'];
    if (temp != null) {
      final tempVal = (temp as num).toDouble();
      crewVitals.add(TrackingVital(
        title: 'Temperatura Corporal',
        value: tempVal.toStringAsFixed(1),
        unit: '°C',
        status: (tempVal >= 36.1 && tempVal <= 37.2) ? 'normal' : 'attention',
        statusLabel: (tempVal >= 36.1 && tempVal <= 37.2) ? 'Normal' : 'Atenção',
        rangeMin: 35,
        rangeMax: 40,
        normalMin: 36.1,
        normalMax: 37.2,
        icon: Icons.thermostat,
      ));
    }

    // Fatigue/Stress from registros_diarios when available
    double fatiguePct = 0;
    if (registro != null) {
      final fadiga = (registro['fadiga_score'] as num?)?.toDouble() ?? 0;
      final estresse = (registro['nivel_estresse_subjetivo'] as num?)?.toDouble() ?? 0;
      fatiguePct = ((fadiga + estresse) / 20.0 * 100.0).clamp(0, 100);
    } else {
      final hrv = t['hrv_ms'];
      final hr2 = t['frequencia_cardiaca_bpm'];
      if (hrv != null && hr2 != null) {
        final hrvVal = (hrv as num).toDouble();
        final hrVal = (hr2 as num).toDouble();
        fatiguePct = ((100 - (hrvVal / 10)) + (hrVal - 60) * 0.5).clamp(0, 100);
      }
    }
    crewVitals.add(TrackingVital(
      title: 'Nível de Fadiga / Estresse',
      value: fatiguePct.round().toString(),
      unit: '%',
      status: fatiguePct > 70 ? 'alert' : (fatiguePct > 40 ? 'attention' : 'normal'),
      statusLabel: fatiguePct > 70 ? 'Alerta' : (fatiguePct > 40 ? 'Atenção' : 'Normal'),
      rangeMin: 0,
      rangeMax: 100,
      normalMin: 0,
      normalMax: 40,
      icon: Icons.psychology_outlined,
    ));

    // Environment metrics
    final environmentMetrics = <TrackingEnvironment>[];

    if (envSummaries.isNotEmpty) {
      final env = envSummaries.first;

      // Radiation
      final rad = env['media_radiacao_msv_hora'];
      if (rad != null) {
        final radVal = (rad as num).toDouble();
        environmentMetrics.add(TrackingEnvironment(
          title: 'Radiação Cósmica',
          value: radVal.toStringAsFixed(2),
          unit: 'mSv/h',
          status: radVal > 1.0 ? 'alert' : 'normal',
          statusLabel: radVal > 1.0 ? 'Alerta: Alta' : 'Normal',
          description: radVal > 1.0 ? 'acima do limiar de 1.0 mSv/h' : null,
          actionText: radVal > 1.0 ? 'Reduzir exposição externa' : null,
          icon: Icons.blur_on,
        ));
      }

      // Cabin Pressure
      final press = env['media_pressao_kpa'];
      if (press != null) {
        final pressVal = (press as num).toDouble();
        environmentMetrics.add(TrackingEnvironment(
          title: 'Pressão da Cabine',
          value: pressVal.toStringAsFixed(1),
          unit: 'kPa',
          status: (pressVal >= 95 && pressVal <= 105) ? 'normal' : 'attention',
          statusLabel: (pressVal >= 95 && pressVal <= 105) ? 'Estável' : 'Variação',
          icon: Icons.speed_outlined,
        ));
      }

      // CO2
      final co2 = env['media_co2_ppm'];
      if (co2 != null) {
        final co2Val = (co2 as num).toDouble();
        final co2Pct = co2Val / 10000;
        environmentMetrics.add(TrackingEnvironment(
          title: 'Nível de CO2',
          value: (co2Pct).toStringAsFixed(2),
          unit: '%',
          status: co2Pct <= 0.5 ? 'normal' : 'attention',
          statusLabel: co2Pct <= 0.5 ? 'Seguro' : 'Elevado',
          icon: Icons.cloud_outlined,
        ));
      }

      // O2 - default
      environmentMetrics.add(TrackingEnvironment(
        title: 'Oxigênio da Cabine (O2)',
        value: '21.1',
        unit: '%',
        status: 'normal',
        statusLabel: 'Normal',
        icon: Icons.science_outlined,
      ));
    } else {
      environmentMetrics.addAll(_getDefaultEnvironmentMetrics());
    }

    return TrackingData(
      crewVitals: crewVitals,
      environmentMetrics: environmentMetrics,
      lastUpdate: DateTime.now().toIso8601String(),
      astronautId: 'AST001',
      currentModule: 'Módulo Principal',
    );
  }

  Future<List<Map<String, dynamic>>> _fetchEnvSummaries() async {
    if (_db == null) return [];
    try {
      return await _db!.rawQuery('''
        SELECT 
          mn.id AS modulo_id,
          mn.nome AS nome_modulo,
          m.id AS missao_id,
          m.nome_missao AS nome_missao,
          COUNT(a.id) AS quantidade_amostras,
          AVG(a.co2_ppm) AS media_co2_ppm,
          MAX(a.co2_ppm) AS max_co2_ppm,
          AVG(a.radiacao_msv_hora) AS media_radiacao_msv_hora,
          MAX(a.radiacao_msv_hora) AS max_radiacao_msv_hora,
          AVG(a.temperatura_cabine_c) AS media_temperatura_cabine_c,
          MIN(a.temperatura_cabine_c) AS min_temperatura_cabine_c,
          MAX(a.temperatura_cabine_c) AS max_temperatura_cabine_c,
          AVG(a.pressao_kpa) AS media_pressao_kpa,
          MIN(a.pressao_kpa) AS min_pressao_kpa,
          MAX(a.pressao_kpa) AS max_pressao_kpa,
          MIN(a.timestamp_utc) AS primeira_amostra_utc,
          MAX(a.timestamp_utc) AS ultima_amostra_utc
        FROM modulos_nave mn
        JOIN missoes m ON m.id = mn.missao_id AND m.deleted_at IS NULL
        LEFT JOIN ambiente_nave a 
          ON a.modulo_id = mn.id
          AND a.deleted_at IS NULL
        WHERE mn.missao_id = (SELECT missao_id FROM telemetria_fisiologica WHERE astronauta_id = 'AST001' ORDER BY timestamp_utc DESC LIMIT 1)
          AND mn.deleted_at IS NULL
        GROUP BY mn.id, mn.nome, m.id, m.nome_missao
        ORDER BY mn.nome
      ''');
    } catch (e) {
      return [];
    }
  }

  List<TrackingEnvironment> _getDefaultEnvironmentMetrics() {
    return [
      TrackingEnvironment(
        title: 'Radiação Cósmica',
        value: '1.85',
        unit: 'mSv/h',
        status: 'alert',
        statusLabel: 'Alerta: Alta',
        description: 'acima do limiar de 1.0 mSv/h',
        actionText: 'Reduzir exposição externa',
        icon: Icons.blur_on,
      ),
      TrackingEnvironment(
        title: 'Pressão da Cabine',
        value: '101.3',
        unit: 'kPa',
        status: 'normal',
        statusLabel: 'Estável',
        icon: Icons.speed_outlined,
      ),
      TrackingEnvironment(
        title: 'Nível de CO2',
        value: '0.38',
        unit: '%',
        status: 'normal',
        statusLabel: 'Seguro',
        icon: Icons.cloud_outlined,
      ),
      TrackingEnvironment(
        title: 'Oxigênio da Cabine (O2)',
        value: '21.1',
        unit: '%',
        status: 'normal',
        statusLabel: 'Normal',
        icon: Icons.science_outlined,
      ),
    ];
  }

  Future<bool> saveCheckin(List<CheckInAnswer> answers) async {
    if (_db == null) {
      try {
        await initialize();
      } catch (_) {
        return false;
      }
    }
    if (_db == null) return false;
    try {
      final today = DateTime.now().toIso8601String().substring(0, 10); // YYYY-MM-DD
      final astronautId = 'AST001';

      double? horasSono;
      int? humorScore;
      int? nivelEstresse;
      int? fadigaScore;
      int? ingestaoAguaMl;
      int? exercicioAerobicoMin;

      for (final a in answers) {
        if (!a.isAnswered || a.selected.isEmpty) continue;
        final sel = a.selected.first;
        switch (a.category) {
          case CheckInCategory.sleep:
            if (sel.contains('< 4')) horasSono = 3.0;
            else if (sel.contains('4–6') || sel.contains('4-6')) horasSono = 5.0;
            else if (sel.contains('6–8') || sel.contains('6-8')) horasSono = 7.0;
            else if (sel.contains('8h')) horasSono = 9.0;
            break;
          case CheckInCategory.mood:
            if (sel.contains('Great')) humorScore = 10;
            else if (sel.contains('Good')) humorScore = 8;
            else if (sel.contains('Okay')) humorScore = 6;
            else if (sel.contains('Low')) humorScore = 4;
            else if (sel.contains('Bad')) humorScore = 2;
            break;
          case CheckInCategory.stress:
            if (sel == 'Low') nivelEstresse = 2;
            else if (sel == 'Medium') nivelEstresse = 5;
            else if (sel == 'High') nivelEstresse = 8;
            break;
          case CheckInCategory.pain:
            if (sel == 'None') fadigaScore = 1;
            else if (sel == 'Mild') fadigaScore = 3;
            else if (sel == 'Moderate') fadigaScore = 5;
            else if (sel == 'Severe') fadigaScore = 8;
            break;
          case CheckInCategory.hydration:
            if (sel.contains('< 1')) ingestaoAguaMl = 750;
            else if (sel.contains('1–2') || sel.contains('1-2')) ingestaoAguaMl = 1500;
            else if (sel.contains('2–3') || sel.contains('2-3')) ingestaoAguaMl = 2500;
            else if (sel.contains('3L')) ingestaoAguaMl = 3500;
            break;
          case CheckInCategory.exercise:
            final mins = int.tryParse(sel);
            if (mins != null) exercicioAerobicoMin = mins;
            break;
          case CheckInCategory.symptoms:
            // No column in registros_diarios; ignore for now
            break;
        }
      }

      // Upsert into registros_diarios
      final now = DateTime.now().toUtc().toIso8601String();
      final existing = await _db!.rawQuery(
        'SELECT id FROM registros_diarios WHERE astronauta_id = ? AND data_registro = ? AND deleted_at IS NULL LIMIT 1',
        [astronautId, today],
      );
      if (existing.isNotEmpty) {
        final id = existing.first['id'] as String;
        await _db!.update(
          'registros_diarios',
          {
            'horas_sono_total': horasSono,
            'humor_score': humorScore,
            'nivel_estresse_subjetivo': nivelEstresse,
            'fadiga_score': fadigaScore,
            'ingestao_agua_ml': ingestaoAguaMl,
            'exercicio_aerobico_min': exercicioAerobicoMin,
            'updated_at': now,
          },
          where: 'id = ?',
          whereArgs: [id],
        );
      } else {
        final id = 'rd_${DateTime.now().millisecondsSinceEpoch}';
        await _db!.insert('registros_diarios', {
          'id': id,
          'astronauta_id': astronautId,
          'data_registro': today,
          'horas_sono_total': horasSono,
          'humor_score': humorScore,
          'nivel_estresse_subjetivo': nivelEstresse,
          'fadiga_score': fadigaScore,
          'ingestao_agua_ml': ingestaoAguaMl,
          'exercicio_aerobico_min': exercicioAerobicoMin,
          'updated_at': now,
        });
      }

      // Refresh data
      await _loadInitialData();
      return true;
    } catch (e) {
      print('Error saving checkin: $e');
      return false;
    }
  }

  TrackingData _getMockData() {
    return TrackingData(
      crewVitals: mockCrewVitals.map((m) => TrackingVital.fromTrackingMetric(m)).toList(),
      environmentMetrics: mockEnvironmentMetrics.map((m) => TrackingEnvironment.fromTrackingMetric(m)).toList(),
      lastUpdate: DateTime.now().toIso8601String(),
      astronautId: 'AST001',
      currentModule: 'Módulo Principal',
    );
  }

  void dispose() {
    _refreshTimer?.cancel();
    _db?.close();
    _dataController?.close();
  }
}