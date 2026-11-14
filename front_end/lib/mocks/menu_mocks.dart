import '../core/models/checkin.dart';

/// MOCK — data exibida no header da Home.
/// TODO: trocar por data real formatada (ex.: pacote `intl`, DateFormat.now()).
const mockTodayLabel = 'Friday, 13 November';

/// MOCK — resumo do check-in do dia, usado em CheckInStatusCard.
const mockDailyCheckInSummary = DailyCheckInSummary(
  categories: [
    CheckInCategoryStatus(category: CheckInCategory.sleep, label: 'Sleep'),
    CheckInCategoryStatus(category: CheckInCategory.mood, label: 'Mood'),
    CheckInCategoryStatus(category: CheckInCategory.stress, label: 'Stress'),
    CheckInCategoryStatus(category: CheckInCategory.pain, label: 'Pain'),
    CheckInCategoryStatus(category: CheckInCategory.hydration, label: 'Hydration'),
    CheckInCategoryStatus(category: CheckInCategory.exercise, label: 'Exercise'),
    CheckInCategoryStatus(category: CheckInCategory.symptoms, label: 'Symptoms'),
  ],
);

/// MOCK — status de conectividade/sincronização, usado em SyncStatusCard.
const mockSyncStatus = SyncStatus(
  isOnline: true,
  isSyncing: true,
  label: 'Sincronizando',
);