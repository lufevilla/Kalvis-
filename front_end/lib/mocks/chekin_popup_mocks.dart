import 'package:flutter/material.dart';
import 'package:front_end/core/models/checkin.dart';
import 'package:front_end/widgets/popups/checkin_popup.dart';

/// MOCK — perguntas do check-in diário, na mesma ordem e opções do preview.
/// TODO(rust): se as opções puderem mudar por configuração, buscar de
/// GET /checkins/questions em vez de usar esta lista fixa.
const mockCheckInQuestions = <CheckInQuestion>[
  CheckInQuestion(
    category: CheckInCategory.sleep,
    label: 'Sleep',
    icon: Icons.bedtime_outlined,
    type: CheckInFieldType.chips,
    options: ['< 4h', '4–6h', '6–8h', '8h+'],
  ),
  CheckInQuestion(
    category: CheckInCategory.mood,
    label: 'Mood',
    icon: Icons.sentiment_satisfied_alt_outlined,
    type: CheckInFieldType.chips,
    options: ['😄 Great', '🙂 Good', '😐 Okay', '😟 Low', '😢 Bad'],
  ),
  CheckInQuestion(
    category: CheckInCategory.stress,
    label: 'Stress',
    icon: Icons.monitor_heart_outlined,
    type: CheckInFieldType.chips,
    options: ['Low', 'Medium', 'High'],
  ),
  CheckInQuestion(
    category: CheckInCategory.pain,
    label: 'Pain',
    icon: Icons.bolt_outlined,
    type: CheckInFieldType.chips,
    options: ['None', 'Mild', 'Moderate', 'Severe'],
  ),
  CheckInQuestion(
    category: CheckInCategory.hydration,
    label: 'Hydration',
    icon: Icons.water_drop_outlined,
    type: CheckInFieldType.chips,
    options: ['< 1L', '1–2L', '2–3L', '3L+'],
  ),
  CheckInQuestion(
    category: CheckInCategory.exercise,
    label: 'Exercise',
    icon: Icons.fitness_center_outlined,
    type: CheckInFieldType.number,
    unitLabel: 'minutos',
  ),
  CheckInQuestion(
    category: CheckInCategory.symptoms,
    label: 'Symptoms',
    icon: Icons.thermostat_outlined,
    type: CheckInFieldType.chips,
    multi: true,
    options: ['None', 'Headache', 'Nausea', 'Fatigue', 'Dizziness', 'Other'],
  ),
];
