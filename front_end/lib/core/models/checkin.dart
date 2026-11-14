/// TODO(rust): estes modelos devem espelhar o contrato da API.
/// O JSON esperado segue o formato abaixo; ajuste os campos quando o Rust
/// definir a resposta real.
///
/// GET /checkins/today ->
///   { "categories": [ { "key": "sleep", "label": "Sleep", "answered": false } ] }

enum CheckInCategory { sleep, mood, stress, pain, hydration, exercise, symptoms }

class CheckInCategoryStatus {
  const CheckInCategoryStatus({
    required this.category,
    required this.label,
    this.answered = false,
  });

  factory CheckInCategoryStatus.fromJson(Map<String, dynamic> json) {
    return CheckInCategoryStatus(
      category: CheckInCategory.values.byName(json['key'] as String),
      label: json['label'] as String,
      answered: json['answered'] as bool? ?? false,
    );
  }

  final CheckInCategory category;
  final String label;
  final bool answered;
}

class DailyCheckInSummary {
  const DailyCheckInSummary({required this.categories});

  factory DailyCheckInSummary.fromJson(Map<String, dynamic> json) {
    final list = json['categories'] as List<dynamic>;
    return DailyCheckInSummary(
      categories: list
          .map((e) => CheckInCategoryStatus.fromJson(e as Map<String, dynamic>))
          .toList(),
    );
  }

  final List<CheckInCategoryStatus> categories;

  int get answeredCount => categories.where((c) => c.answered).length;
  int get totalCount => categories.length;
}

class SyncStatus {
  const SyncStatus({
    required this.isOnline,
    required this.isSyncing,
    required this.label,
  });

  final bool isOnline;
  final bool isSyncing;
  final String label;
}
