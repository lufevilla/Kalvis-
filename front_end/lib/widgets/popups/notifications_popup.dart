import 'package:flutter/material.dart';

import '../../core/app_colors.dart';
import '../../mocks/popup_mocks.dart';

/// Popup de notificações, pensado para abrir ancorado no sino do header.
///
/// Usa `Material` como raiz para que os `InkWell` funcionem em qualquer
/// lugar onde for aberto (dialog, overlay, menu).
/// A lógica de abrir/fechar e de posicionamento fica com quem chama.
class NotificationsPopup extends StatelessWidget {
  const NotificationsPopup({super.key});

  @override
  Widget build(BuildContext context) {
    return Material(
      color: AppColors.navy,
      borderRadius: BorderRadius.circular(16),
      clipBehavior: Clip.antiAlias,
      child: SizedBox(
        width: 280,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Padding(
              padding: EdgeInsets.fromLTRB(16, 16, 16, 8),
              child: Text(
                'Notificações',
                style: TextStyle(
                  color: Colors.white,
                  fontSize: 16,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ),
            // TODO: lista real de notificações, com paginação/scroll se crescer
            for (final notification in mockNotifications)
              _NotificationTile(data: notification),
            const SizedBox(height: 8),
          ],
        ),
      ),
    );
  }
}

class _NotificationTile extends StatelessWidget {
  const _NotificationTile({required this.data});

  final NotificationData data;

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: () {
        // TODO: marcar notificação como lida / abrir detalhe
      },
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            CircleAvatar(
              radius: 16,
              backgroundColor: AppColors.navyLight,
              child: Icon(data.icon, size: 16, color: Colors.white),
            ),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Text(
                        data.title,
                        style: const TextStyle(
                          color: Colors.white,
                          fontSize: 13,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      Text(
                        data.time,
                        style: const TextStyle(
                          color: AppColors.navyMuted,
                          fontSize: 11,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 2),
                  Text(
                    data.subtitle,
                    style: const TextStyle(
                      color: AppColors.navyMuted,
                      fontSize: 12,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}