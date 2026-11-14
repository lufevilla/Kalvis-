import 'package:flutter/material.dart';
import 'package:front_end/mocks/menu_mocks.dart';
import 'package:front_end/screens/home/widgets/checkin_status_card.dart';
import 'package:front_end/screens/home/widgets/daily_checkin_card.dart';
import 'package:front_end/screens/home/widgets/sync_status_card.dart';

/// Conteúdo da Home. Sidebar e header vêm do AppShell, não daqui.
/// Todos os dados são mocks (ver lib/mocks/home_mocks.dart).
class HomeScreen extends StatelessWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final mainColumn = Column(
      children: [
        DailyCheckInCard(
          // TODO(rust): iniciar fluxo de check-in por texto
          onStartCheckIn: () {

          },
          // TODO(rust): iniciar fluxo de check-in por voz
          onAnswerByVoice: () {},
        ),
        const SizedBox(height: 20),
        CheckInStatusCard(summary: mockDailyCheckInSummary),
      ],
    );

    final sideColumn = SyncStatusCard(status: mockSyncStatus);

    return LayoutBuilder(
      builder: (context, constraints) {
        final isWide = constraints.maxWidth >= 900;

        return SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: isWide
              ? Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(flex: 2, child: mainColumn),
              const SizedBox(width: 20),
              Expanded(flex: 1, child: sideColumn),
            ],
          )
              : Column(
            children: [
              mainColumn,
              const SizedBox(height: 20),
              sideColumn,
            ],
          ),
        );
      },
    );
  }
}
