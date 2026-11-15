import 'package:flutter/material.dart';
import 'package:front_end/core/models/checkin.dart';
import 'package:front_end/mocks/chekin_popup_mocks.dart';
import 'package:front_end/mocks/menu_mocks.dart';
import 'package:front_end/widgets/checkin/checkin_entry_card.dart';
import 'package:front_end/widgets/popups/checkin_popup.dart';

/// Conteúdo da Home. Sidebar e header vêm do AppShell, não daqui.
/// Dados mockados por enquanto (ver lib/mocks/).
class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key});

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  // TODO(rust): trocar pelo status real de conexão com o backend
  final SyncStatus _syncStatus = mockSyncStatus;

  int _answeredCount = 0;

  void _onAnswersSubmitted(List<CheckInAnswer> answers) {
    // TODO(rust): enviar as respostas para o backend, ex.: POST /checkins/today
    // Sugestão de payload: answers.map((a) => {'key': a.category.name, 'selected': a.selected})
    setState(() {
      _answeredCount = answers.where((a) => a.isAnswered).length;
    });
  }

  @override
  Widget build(BuildContext context) {
    final mainColumn = Column(
      children: [
        CheckInEntryCard(
          answeredCount: _answeredCount,
          totalCount: mockCheckInQuestions.length,
          onAnswersSubmitted: _onAnswersSubmitted,
        ),
      ],
    );

    return LayoutBuilder(
      builder: (context, constraints) {
        final isWide = constraints.maxWidth >= 900;

        return SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: isWide
              ? Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [Expanded(flex: 2, child: mainColumn)],
                )
              : Column(children: [mainColumn, const SizedBox(height: 20)]),
        );
      },
    );
  }
}
