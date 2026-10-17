import 'package:flutter/material.dart';

/// Brand seed color; every other color is derived from it by Material 3.
const Color ecSeedColor = Color(0xFF2F6FED);

/// Light theme shared by all apps.
ThemeData ecLightTheme() => ThemeData(colorSchemeSeed: ecSeedColor);

/// Dark theme shared by all apps.
ThemeData ecDarkTheme() =>
    ThemeData(colorSchemeSeed: ecSeedColor, brightness: Brightness.dark);
