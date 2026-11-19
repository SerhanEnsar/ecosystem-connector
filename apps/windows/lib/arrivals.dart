import 'dart:io';

import 'package:ec_app/ec_app.dart';
import 'package:ec_ui/ec_ui.dart';
import 'package:file_selector/file_selector.dart';
import 'package:flutter/services.dart';
import 'package:url_launcher/url_launcher.dart';

const _imageExtensions = {'.png', '.jpg', '.jpeg', '.gif', '.webp', '.bmp'};

/// Turns something received into the card the window shows, with the buttons the
/// macOS card has: "Show in folder" or "Open in browser" or "Copy", plus "Open" and
/// "Save to…" for a single file.
ArrivalEntry windowsArrival(ArrivalInfo arrival) {
  if (arrival.url case final url?) {
    return ArrivalEntry(
      data: ArrivalCardData(
        title: arrival.displayTitle,
        from: arrival.from,
        detail: Uri.tryParse(url)?.host ?? url,
        isPage: true,
      ),
      onPrimary: () => _openPage(url),
    );
  }
  if (arrival.text case final text?) {
    final lines = text.split('\n').where((l) => l.trim().isNotEmpty).toList();
    return ArrivalEntry(
      data: ArrivalCardData(
        title: arrival.displayTitle,
        from: arrival.from,
        detail: lines.skip(1).join(' ').trim(),
        isText: true,
      ),
      onPrimary: () => Clipboard.setData(ClipboardData(text: text)),
    );
  }

  final paths = arrival.paths;
  final single = paths.length == 1 ? paths.single : null;
  final image = single != null && _isImage(single) ? single : null;
  return ArrivalEntry(
    data: ArrivalCardData(
      title: arrival.displayTitle,
      from: arrival.from,
      detail: formatBytes(arrival.size),
      imagePath: image,
      canSaveElsewhere: single != null,
      linked: arrival.linked,
    ),
    onPrimary: () => _showInFolder(paths.first),
    onOpen: single == null ? null : () => _openFile(single),
    onSaveElsewhere: single == null ? null : () => _saveTo(single),
  );
}

bool _isImage(String path) {
  final dot = path.lastIndexOf('.');
  return dot >= 0 &&
      _imageExtensions.contains(path.substring(dot).toLowerCase());
}

Future<void> _openPage(String url) =>
    launchUrl(Uri.parse(url), mode: LaunchMode.externalApplication);

Future<void> _openFile(String path) => launchUrl(Uri.file(path));

/// Opens Explorer with [path] selected.
Future<void> _showInFolder(String path) async {
  await Process.run('explorer.exe', ['/select,$path']);
}

/// Asks where to keep [path], then moves it there and shows it in Explorer.
Future<void> _saveTo(String path) async {
  final name = path.split(RegExp(r'[/\\]')).last;
  final target = await getSaveLocation(suggestedName: name);
  final to = target?.path;
  if (to == null) return;
  try {
    await File(path).rename(to);
  } on FileSystemException {
    // Another drive: copy, then remove the original.
    await File(path).copy(to);
    await File(path).delete();
  }
  await _showInFolder(to);
}
