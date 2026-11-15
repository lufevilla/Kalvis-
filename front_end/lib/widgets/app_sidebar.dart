import 'package:flutter/material.dart';

import '../../core/app_colors.dart';

/// Sidebar fixa e clicável.
///
/// Não guarda estado próprio: quem usa informa qual item está ativo
/// (`activeLabel`) e recebe o clique (`onItemSelected`).
class AppSidebar extends StatelessWidget {
  const AppSidebar({
    super.key,
    required this.activeLabel,
    required this.onItemSelected,
  });

  final String activeLabel;
  final ValueChanged<String> onItemSelected;

  static const _navItems = <({String label, IconData icon})>[
    (label: 'Home', icon: Icons.home_outlined),
    (label: 'Tracking', icon: Icons.map_outlined),
  ];

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 220,
      color: AppColors.navy,
      padding: const EdgeInsets.fromLTRB(12, 24, 12, 24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const _Brand(),
          const SizedBox(height: 32),
          for (final item in _navItems)
            _NavTile(
              label: item.label,
              icon: item.icon,
              isActive: item.label == activeLabel,
              onTap: () => onItemSelected(item.label),
            ),
        ],
      ),
    );
  }
}

class _Brand extends StatelessWidget {
  const _Brand();

  @override
  Widget build(BuildContext context) {
    return const Padding(
      padding: EdgeInsets.symmetric(horizontal: 8),
      child: Row(
        children: [
          // TODO: trocar pelo logo real do app
          CircleAvatar(radius: 16, backgroundColor: AppColors.navyLight),
          SizedBox(width: 10),
          Text(
            'Kalvis',
            style: TextStyle(
              color: Colors.white,
              fontSize: 18,
              fontWeight: FontWeight.w700,
            ),
          ),
        ],
      ),
    );
  }
}

class _NavTile extends StatelessWidget {
  const _NavTile({
    required this.label,
    required this.icon,
    required this.isActive,
    required this.onTap,
  });

  final String label;
  final IconData icon;
  final bool isActive;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final color = isActive ? Colors.white : AppColors.navyMuted;

    return Padding(
      padding: const EdgeInsets.only(bottom: 4),
      child: Material(
        color: isActive ? AppColors.navyLight : Colors.transparent,
        borderRadius: BorderRadius.circular(10),
        child: InkWell(
          borderRadius: BorderRadius.circular(10),
          hoverColor: AppColors.navyLight,
          highlightColor: AppColors.navyLight,
          mouseCursor: SystemMouseCursors.click,
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            child: Row(
              children: [
                Icon(icon, size: 20, color: color),
                const SizedBox(width: 12),
                Text(
                  label,
                  style: TextStyle(
                    color: color,
                    fontSize: 14,
                    fontWeight: FontWeight.w500,
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _UserRow extends StatelessWidget {
  const _UserRow({required this.name});

  final String name;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      child: Row(
        children: [
          const CircleAvatar(
            radius: 16,
            backgroundColor: AppColors.navyLight,
            child: Icon(Icons.person, size: 18, color: Colors.white),
          ),
          const SizedBox(width: 10),
          Text(
            name,
            style: const TextStyle(
              color: Colors.white,
              fontSize: 14,
              fontWeight: FontWeight.w500,
            ),
          ),
        ],
      ),
    );
  }
}
