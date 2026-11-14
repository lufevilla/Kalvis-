import 'package:flutter/material.dart';
import 'package:front_end/widgets/popups/notifications_popup.dart';

import '../../core/app_colors.dart';

class AppHeader extends StatefulWidget {
  const AppHeader({
    super.key,
    required this.title,
    required this.dateLabel,
  });

  final String title;
  final String dateLabel;

  @override
  State<AppHeader> createState() => _AppHeaderState();
}

class _AppHeaderState extends State<AppHeader> {
  final LayerLink _notificationLink = LayerLink();

  OverlayEntry? _notificationOverlay;

  void _toggleNotifications() {
    if (_notificationOverlay != null) {
      _hideNotifications();
    } else {
      _showNotifications();
    }
  }

  void _showNotifications() {
    _notificationOverlay = OverlayEntry(
      builder: (context) {
        return Stack(
          children: [
            // Fecha ao clicar fora do popup.
            Positioned.fill(
              child: GestureDetector(
                behavior: HitTestBehavior.translucent,
                onTap: _hideNotifications,
                child: const SizedBox.expand(),
              ),
            ),

            // Popup ancorado ao sino.
            Positioned(
              width: 280,
              child: CompositedTransformFollower(
                link: _notificationLink,
                showWhenUnlinked: false,
                targetAnchor: Alignment.bottomRight,
                followerAnchor: Alignment.topRight,
                offset: const Offset(0, 8),
                child: const NotificationsPopup(),
              ),
            ),
          ],
        );
      },
    );

    Overlay.of(context, rootOverlay: true).insert(_notificationOverlay!);
  }

  void _hideNotifications() {
    _notificationOverlay?.remove();
    _notificationOverlay = null;
  }

  @override
  void dispose() {
    _hideNotifications();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 72,
      padding: const EdgeInsets.symmetric(horizontal: 24),
      decoration: const BoxDecoration(
        color: AppColors.surface,
        border: Border(bottom: BorderSide(color: AppColors.border)),
      ),
      child: Row(
        children: [
          Column(
            mainAxisAlignment: MainAxisAlignment.center,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                widget.title,
                style: const TextStyle(
                  fontSize: 20,
                  fontWeight: FontWeight.w700,
                  color: AppColors.navy,
                ),
              ),
              Text(
                widget.dateLabel,
                style: const TextStyle(
                  fontSize: 13,
                  color: AppColors.textSubtle,
                ),
              ),
            ],
          ),
          const Spacer(),

          // Notificações
          CompositedTransformTarget(
            link: _notificationLink,
            child: IconButton(
              onPressed: _toggleNotifications,
              icon: const Icon(
                Icons.notifications_none,
                color: AppColors.navy,
              ),
              style: IconButton.styleFrom(
                side: const BorderSide(color: AppColors.border),
                shape: const CircleBorder(),
              ),
            ),
          ),

          const SizedBox(width: 10),

          _HeaderPill(
            icon: Icons.wifi,
            label: 'Synchronized',
            onTap: () {
              // TODO: abrir detalhe de conexão
            },
          ),

          const SizedBox(width: 10),

          _HeaderPill(
            icon: Icons.call_outlined,
            label: 'Emergency',
            accent: true,
            onTap: () {
              // TODO: acionar fluxo de emergência
            },
          ),
        ],
      ),
    );
  }
}

class _HeaderPill extends StatelessWidget {
  const _HeaderPill({
    required this.icon,
    required this.label,
    required this.onTap,
    this.accent = false,
  });

  final IconData icon;
  final String label;
  final VoidCallback onTap;
  final bool accent;

  @override
  Widget build(BuildContext context) {
    final color = accent ? AppColors.accent : AppColors.navy;

    return Material(
      color: accent ? AppColors.accentSoft : AppColors.surface,
      shape: StadiumBorder(
        side: BorderSide(color: accent ? AppColors.accentBorder : AppColors.border),
      ),
      child: InkWell(
        customBorder: const StadiumBorder(),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(icon, size: 16, color: color),
              const SizedBox(width: 6),
              Text(
                label,
                style: TextStyle(
                  fontSize: 14,
                  fontWeight: FontWeight.w500,
                  color: color,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
