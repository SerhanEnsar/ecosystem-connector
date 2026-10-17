import 'package:flutter/material.dart';

/// Brand colors. The cyan is the "link" between devices; ember marks data in motion
/// and is used only while a transfer runs.
abstract final class EcColors {
  static const ink = Color(0xFF14213D);
  static const paper = Color(0xFFF3F6FA);
  static const link = Color(0xFF0FA3B1);
  static const ember = Color(0xFFFF8552);

  static const inkDark = Color(0xFFE6EDF5);
  static const paperDark = Color(0xFF0E1726);
  static const linkDark = Color(0xFF2CC6D4);
  static const emberDark = Color(0xFFFF9466);
}

/// Extra colors the screens need beyond the Material scheme.
@immutable
class EcPalette extends ThemeExtension<EcPalette> {
  const EcPalette({
    required this.link,
    required this.ember,
    required this.muted,
  });

  /// Online devices and the connection spine.
  final Color link;

  /// Data in motion.
  final Color ember;

  /// Offline devices and secondary lines.
  final Color muted;

  static EcPalette of(BuildContext context) =>
      Theme.of(context).extension<EcPalette>()!;

  @override
  EcPalette copyWith({Color? link, Color? ember, Color? muted}) => EcPalette(
    link: link ?? this.link,
    ember: ember ?? this.ember,
    muted: muted ?? this.muted,
  );

  @override
  EcPalette lerp(EcPalette? other, double t) {
    if (other == null) return this;
    return EcPalette(
      link: Color.lerp(link, other.link, t)!,
      ember: Color.lerp(ember, other.ember, t)!,
      muted: Color.lerp(muted, other.muted, t)!,
    );
  }
}

/// Monospace style for ids and pairing codes.
const ecMono = TextStyle(
  fontFamily: 'monospace',
  fontFamilyFallback: ['Menlo', 'Consolas', 'Roboto Mono'],
  fontFeatures: [FontFeature.tabularFigures()],
);

ThemeData _build(Brightness brightness) {
  final dark = brightness == Brightness.dark;
  final scheme =
      ColorScheme.fromSeed(
        seedColor: EcColors.link,
        brightness: brightness,
      ).copyWith(
        primary: dark ? EcColors.linkDark : EcColors.link,
        surface: dark ? EcColors.paperDark : EcColors.paper,
        onSurface: dark ? EcColors.inkDark : EcColors.ink,
      );
  return ThemeData(
    colorScheme: scheme,
    scaffoldBackgroundColor: scheme.surface,
    extensions: [
      EcPalette(
        link: dark ? EcColors.linkDark : EcColors.link,
        ember: dark ? EcColors.emberDark : EcColors.ember,
        muted: scheme.onSurface.withValues(alpha: 0.32),
      ),
    ],
  );
}

/// Light theme shared by all apps.
ThemeData ecLightTheme() => _build(Brightness.light);

/// Dark theme shared by all apps.
ThemeData ecDarkTheme() => _build(Brightness.dark);
