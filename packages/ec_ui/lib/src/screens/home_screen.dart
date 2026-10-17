import 'package:flutter/material.dart';

import '../controller.dart';
import '../model.dart';
import '../platform.dart';
import '../widgets/device_spine.dart';
import '../widgets/receive_banner.dart';
import '../widgets/share_bar.dart';
import '../widgets/vpn_notice.dart';
import '../fling/fling_setup_card.dart';
import 'pairing_sheet.dart';
import 'send_sheet.dart';

/// The main screen: this device, paired devices, and incoming transfers.
class HomeScreen extends StatefulWidget {
  const HomeScreen({
    super.key,
    required this.controller,
    required this.platform,
  });

  final EcController controller;
  final EcPlatform platform;

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  var _vpnDismissed = false;
  var _flingDismissed = false;

  EcController get controller => widget.controller;

  Future<void> _forget(BuildContext context, EcDevice device) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('Forget ${device.name}?'),
        content: const Text(
          'It will not be able to connect until you pair again.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('Forget'),
          ),
        ],
      ),
    );
    if (confirmed ?? false) await controller.forget(device);
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: controller,
      builder: (context, _) {
        final receiving = controller.receiving;
        final text = Theme.of(context).textTheme;
        return Scaffold(
          appBar: AppBar(title: const Text('Ecosystem Connector')),
          body: Stack(
            children: [
              DeviceSpine(
                myName: controller.myName,
                myShortId: controller.myShortId,
                devices: controller.devices,
                busy: receiving != null,
                onTap: (device) => pickAndSend(context, controller, device),
                onForget: (device) => _forget(context, device),
              ),
              Align(
                alignment: Alignment.topCenter,
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    if (controller.flingSetup case final setup?
                        when !setup.ready && !_flingDismissed)
                      FlingSetupCard(
                        controller: controller,
                        setup: setup,
                        onDismiss: () => setState(() => _flingDismissed = true),
                      ),
                    if (controller.vpnActive && !_vpnDismissed)
                      VpnNotice(
                        onDismiss: () => setState(() => _vpnDismissed = true),
                      ),
                    if (controller.pendingShare.isNotEmpty)
                      ShareBar(
                        count: controller.pendingShare.length,
                        devices: controller.devices,
                        onDismiss: controller.dismissShare,
                        onSend: (device) {
                          final paths = controller.pendingShare;
                          controller.dismissShare();
                          sendWithProgress(context, controller, device, paths);
                        },
                      ),
                  ],
                ),
              ),
              if (controller.devices.isEmpty)
                Align(
                  alignment: const Alignment(0, 0.15),
                  child: Padding(
                    padding: const EdgeInsets.all(32),
                    child: Text(
                      'No paired devices yet. Pair one to start sending files.',
                      style: text.bodyLarge,
                      textAlign: TextAlign.center,
                    ),
                  ),
                ),
              if (receiving != null)
                Align(
                  alignment: Alignment.bottomCenter,
                  child: ReceiveBanner(status: receiving),
                ),
            ],
          ),
          floatingActionButton: FloatingActionButton.extended(
            onPressed: () => showPairingSheet(context, controller),
            icon: const Icon(Icons.add_link),
            label: const Text('Pair a device'),
          ),
        );
      },
    );
  }
}
