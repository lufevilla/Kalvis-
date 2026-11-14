final _db = <String, List<Map<String, num>>>{
  'AST001': [
    {'mission_day': 137, 'heart_rate': 72, 'sleep_hours': 7.1, 'stress_score': 4, 'loneliness_score': 5, 'spo2': 98},
    {'mission_day': 138, 'heart_rate': 74, 'sleep_hours': 6.8, 'stress_score': 4, 'loneliness_score': 5, 'spo2': 98},
    {'mission_day': 139, 'heart_rate': 75, 'sleep_hours': 6.3, 'stress_score': 5, 'loneliness_score': 6, 'spo2': 97},
    {'mission_day': 140, 'heart_rate': 85, 'sleep_hours': 5.8, 'stress_score': 7, 'loneliness_score': 8, 'spo2': 97},
    {'mission_day': 141, 'heart_rate': 88, 'sleep_hours': 5.5, 'stress_score': 7, 'loneliness_score': 8, 'spo2': 97},
  ],
};

List<Map<String, num>> buscarHistorico(String id, String indicador, {int dias = 7}) {
  final r = (_db[id] ?? []).where((x) => x[indicador] != null).toList();
  return r.sublist(r.length > dias ? r.length - dias : 0)
      .map((x) => {'mission_day': x['mission_day']!, 'valor': x[indicador]!})
      .toList();
}
