import 'package:flutter/widgets.dart';

class NextmoePowStyle {
  const NextmoePowStyle({
    required this.background,
    required this.foreground,
    required this.muted,
    required this.border,
    required this.track,
    required this.accent,
    required this.success,
    required this.danger,
    this.radius = 8,
    this.padding = const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
    this.textStyle = const TextStyle(fontSize: 14, height: 1.4),
  });

  final Color background;
  final Color foreground;
  final Color muted;
  final Color border;
  final Color track;
  final Color accent;
  final Color success;
  final Color danger;
  final double radius;
  final EdgeInsets padding;
  final TextStyle textStyle;

  static const light = NextmoePowStyle(
    background: Color(0xFFFFFFFF),
    foreground: Color(0xFF18181B),
    muted: Color(0xFF71717A),
    border: Color(0xFFE4E4E7),
    track: Color(0xFFF4F4F5),
    accent: Color(0xFF2563EB),
    success: Color(0xFF16A34A),
    danger: Color(0xFFDC2626),
  );

  static const dark = NextmoePowStyle(
    background: Color(0xFF18181B),
    foreground: Color(0xFFF4F4F5),
    muted: Color(0xFFA1A1AA),
    border: Color(0xFF3F3F46),
    track: Color(0xFF27272A),
    accent: Color(0xFF60A5FA),
    success: Color(0xFF4ADE80),
    danger: Color(0xFFF87171),
  );

  static NextmoePowStyle of(BuildContext context) =>
      MediaQuery.maybePlatformBrightnessOf(context) == Brightness.dark ? dark : light;
}
