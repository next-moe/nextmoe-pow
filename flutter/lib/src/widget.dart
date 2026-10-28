import 'package:flutter/widgets.dart';

import 'controller.dart';
import 'messages.dart';
import 'style.dart';

/// The visible bot check. It starts [controller] when first built; the app
/// reads [PowController.solution] or awaits [PowController.execute].
class NextmoePow extends StatefulWidget {
  const NextmoePow({super.key, required this.controller, this.style, this.locale});

  final PowController controller;
  final NextmoePowStyle? style;
  final Locale? locale;

  @override
  State<NextmoePow> createState() => _NextmoePowState();
}

class _NextmoePowState extends State<NextmoePow> {
  @override
  void initState() {
    super.initState();
    if (widget.controller.state == PowState.idle) widget.controller.execute().ignore();
  }

  @override
  Widget build(BuildContext context) {
    final style = widget.style ?? NextmoePowStyle.of(context);
    final messages = PowMessages.of(widget.locale ?? Localizations.maybeLocaleOf(context) ?? View.of(context).platformDispatcher.locale);
    final animate = !(MediaQuery.maybeDisableAnimationsOf(context) ?? false);
    return ListenableBuilder(
      listenable: widget.controller,
      builder: (context, _) {
        final c = widget.controller;
        final text = switch (c.state) {
          PowState.idle => '',
          PowState.verifying => c.renewing ? messages.expired : messages.verifying,
          PowState.solved => messages.verified,
          PowState.error => messages.failed,
        };
        final markColor = switch (c.state) {
          PowState.solved => style.success,
          PowState.error => style.danger,
          _ => null,
        };
        return Semantics(
          container: true,
          label: messages.label,
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: style.background,
              border: Border.all(color: style.border),
              borderRadius: BorderRadius.circular(style.radius),
            ),
            child: Padding(
              padding: style.padding,
              child: DefaultTextStyle.merge(
                style: style.textStyle.copyWith(color: style.foreground),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Row(children: [
                      _Mark(color: markColor, outline: style.muted),
                      const SizedBox(width: 8),
                      Expanded(child: Semantics(liveRegion: true, child: Text(text))),
                      if (c.state == PowState.error) _RetryButton(label: messages.retry, style: style, onPressed: c.reset),
                    ]),
                    if (c.state != PowState.error) ...[
                      const SizedBox(height: 8),
                      _Bar(
                        value: c.progress,
                        color: c.state == PowState.solved ? style.success : style.accent,
                        track: style.track,
                        animate: animate,
                        label: messages.label,
                      ),
                    ],
                  ],
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}

class _Mark extends StatelessWidget {
  const _Mark({required this.color, required this.outline});

  final Color? color;
  final Color outline;

  @override
  Widget build(BuildContext context) => ExcludeSemantics(
        child: Container(
          width: 16,
          height: 16,
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            color: color,
            border: Border.all(color: color ?? outline, width: 2),
          ),
        ),
      );
}

class _Bar extends StatelessWidget {
  const _Bar({required this.value, required this.color, required this.track, required this.animate, required this.label});

  final double value;
  final Color color;
  final Color track;
  final bool animate;
  final String label;

  @override
  Widget build(BuildContext context) => Semantics(
        label: label,
        value: '${(value * 100).round()}%',
        child: ClipRRect(
          borderRadius: BorderRadius.circular(2),
          child: SizedBox(
            height: 4,
            child: ColoredBox(
              color: track,
              child: Align(
                alignment: AlignmentDirectional.centerStart,
                child: TweenAnimationBuilder<double>(
                  tween: Tween(end: value),
                  duration: animate ? const Duration(milliseconds: 120) : Duration.zero,
                  // Align passes loose constraints, so without heightFactor the childless
                  // ColoredBox collapsed to 0 px and the bar never showed on a device.
                  builder: (context, v, _) => FractionallySizedBox(widthFactor: v, heightFactor: 1, child: ColoredBox(color: color)),
                ),
              ),
            ),
          ),
        ),
      );
}

class _RetryButton extends StatefulWidget {
  const _RetryButton({required this.label, required this.style, required this.onPressed});

  final String label;
  final NextmoePowStyle style;
  final VoidCallback onPressed;

  @override
  State<_RetryButton> createState() => _RetryButtonState();
}

class _RetryButtonState extends State<_RetryButton> {
  bool _focused = false;

  @override
  Widget build(BuildContext context) {
    final s = widget.style;
    return FocusableActionDetector(
      onShowFocusHighlight: (focused) => setState(() => _focused = focused),
      actions: {ActivateIntent: CallbackAction<ActivateIntent>(onInvoke: (_) => widget.onPressed())},
      mouseCursor: SystemMouseCursors.click,
      child: Semantics(
        button: true,
        onTap: widget.onPressed,
        child: GestureDetector(
          onTap: widget.onPressed,
          child: Container(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 2),
            decoration: BoxDecoration(
              border: Border.all(color: s.accent, width: _focused ? 2 : 1),
              borderRadius: BorderRadius.circular(s.radius),
            ),
            child: Text(widget.label, style: TextStyle(color: s.accent)),
          ),
        ),
      ),
    );
  }
}
