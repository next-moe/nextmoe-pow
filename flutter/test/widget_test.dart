import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:nextmoe_pow/nextmoe_pow.dart';
import 'package:nextmoe_pow/src/controller.dart' show refreshDelay;

SolveEvent event({int solved = 0, int attempts = 0, String? solution, int? expiresAt}) => SolveEvent(
      solved: solved,
      k: 4,
      attempts: attempts,
      expectedAttempts: 400,
      elapsedMs: 1,
      expiresAt: expiresAt ?? DateTime.now().millisecondsSinceEpoch ~/ 1000 + 300,
      solution: solution,
    );

class FakeServer {
  int fetches = 0;
  final solves = <StreamController<SolveEvent>>[];
  Object? failNext;

  Future<String> fetch() async {
    fetches++;
    final error = failNext;
    failNext = null;
    if (error != null) throw error;
    return 'token-$fetches';
  }

  Stream<SolveEvent> solve(String token) {
    final c = StreamController<SolveEvent>();
    solves.add(c);
    return c.stream;
  }
}

Widget app(PowController controller, {Locale? locale}) => Directionality(
      textDirection: TextDirection.ltr,
      child: MediaQuery(
        data: const MediaQueryData(),
        child: Center(child: SizedBox(width: 320, child: NextmoePow(controller: controller, locale: locale))),
      ),
    );

Future<void> finish(WidgetTester tester, PowController controller) async {
  await tester.pumpWidget(const SizedBox());
  controller.dispose();
}

void main() {
  late FakeServer server;
  late PowController controller;

  setUp(() {
    server = FakeServer();
    controller = PowController(fetchChallenge: server.fetch, solver: server.solve);
  });

  testWidgets('verifies, reports progress and exposes the solution', (tester) async {
    await tester.pumpWidget(app(controller));
    await tester.pump();
    expect(find.text('Checking your device…'), findsOneWidget);
    server.solves.single.add(event(solved: 2, attempts: 100));
    await tester.pumpAndSettle();
    expect(controller.progress, 0.5);
    server.solves.single.add(event(solved: 4, attempts: 300, solution: 'token-1.nonces'));
    await tester.pumpAndSettle();
    expect(find.text('Verified'), findsOneWidget);
    final bar = tester.getSize(find.descendant(of: find.byType(FractionallySizedBox), matching: find.byType(ColoredBox)));
    final track = tester.getSize(find.ancestor(of: find.byType(Align), matching: find.byType(ColoredBox)).first);
    expect(bar, track);
    expect(bar.height, 4);
    expect(controller.solution, 'token-1.nonces');
    expect(await controller.execute(), 'token-1.nonces');
    await finish(tester, controller);
  });

  testWidgets('speaks Chinese and Japanese', (tester) async {
    await tester.pumpWidget(app(controller, locale: const Locale('zh')));
    await tester.pump();
    expect(find.text('正在验证设备…'), findsOneWidget);
    await tester.pumpWidget(app(controller, locale: const Locale('ja')));
    expect(find.text('デバイスを確認しています…'), findsOneWidget);
    await finish(tester, controller);
  });

  testWidgets('a failed fetch shows retry, and retry solves', (tester) async {
    server.failNext = Exception('offline');
    await tester.pumpWidget(app(controller));
    await tester.pump();
    expect(find.text('Verification failed'), findsOneWidget);
    await tester.tap(find.text('Retry'));
    await tester.pump();
    expect(find.text('Checking your device…'), findsOneWidget);
    server.solves.single.add(event(solved: 4, solution: 's'));
    await tester.pumpAndSettle();
    expect(find.text('Verified'), findsOneWidget);
    expect(server.fetches, 2);
    await finish(tester, controller);
  });

  testWidgets('a caller waiting across reset gets the next solution', (tester) async {
    await tester.pumpWidget(app(controller));
    await tester.pump();
    final waiting = controller.execute();
    controller.reset();
    await tester.pump();
    expect(server.solves, hasLength(2));
    server.solves.last.add(event(solved: 4, solution: 'second'));
    await tester.pumpAndSettle();
    expect(await waiting, 'second');
    await finish(tester, controller);
  });

  testWidgets('renews before the challenge expires', (tester) async {
    await tester.pumpWidget(app(controller));
    await tester.pump();
    final expiresAt = DateTime.now().millisecondsSinceEpoch ~/ 1000 + 60;
    server.solves.single.add(event(solved: 4, solution: 'first', expiresAt: expiresAt));
    await tester.pumpAndSettle();
    await tester.pump(const Duration(seconds: 51));
    expect(controller.solution, isNull);
    expect(find.text('Expired, checking again…'), findsOneWidget);
    expect(server.fetches, 2);
    server.solves.last.add(event(solved: 4, solution: 'second'));
    await tester.pumpAndSettle();
    expect(controller.solution, 'second');
    await finish(tester, controller);
  });

  test('refresh delay never loops on a skewed clock', () {
    final now = DateTime.fromMillisecondsSinceEpoch(1000000 * 1000);
    expect(refreshDelay(1000300, now), const Duration(seconds: 290));
    expect(refreshDelay(999000, now), const Duration(seconds: 30));
  });
}
