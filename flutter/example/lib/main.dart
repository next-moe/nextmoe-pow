import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:nextmoe_pow/nextmoe_pow.dart';

const server = String.fromEnvironment('POW_DEMO_URL', defaultValue: 'http://127.0.0.1:8787');
const autoBench = bool.fromEnvironment('AUTO_BENCH');
const challengeQuery = 'alg=2&m=512&t=1&k=8&d=1000&site=flutter&action=demo';

final _http = HttpClient();

Future<String> fetchChallenge() async {
  final res = await (await _http.getUrl(Uri.parse('$server/api/challenge?$challengeQuery'))).close();
  return (jsonDecode(await utf8.decodeStream(res)) as Map)['token'] as String;
}

Future<Map<String, dynamic>> verify(String solution) async {
  final req = await _http.postUrl(Uri.parse('$server/api/verify'));
  req.headers.contentType = ContentType.json;
  req.write(jsonEncode({'solution': solution, 'site': 'flutter', 'action': 'demo'}));
  return jsonDecode(await utf8.decodeStream(await req.close())) as Map<String, dynamic>;
}

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await initNextmoePow();
  runApp(const ExampleApp());
}

class ExampleApp extends StatelessWidget {
  const ExampleApp({super.key});

  @override
  Widget build(BuildContext context) => MaterialApp(
        title: 'nextmoe_pow',
        theme: ThemeData(colorSchemeSeed: const Color(0xFF2563EB)),
        darkTheme: ThemeData(colorSchemeSeed: const Color(0xFF2563EB), brightness: Brightness.dark),
        home: const ExamplePage(),
      );
}

class ExamplePage extends StatefulWidget {
  const ExamplePage({super.key});

  @override
  State<ExamplePage> createState() => _ExamplePageState();
}

class _ExamplePageState extends State<ExamplePage> {
  final controller = PowController(fetchChallenge: fetchChallenge);
  String result = '';

  @override
  void initState() {
    super.initState();
    if (autoBench) WidgetsBinding.instance.addPostFrameCallback((_) => bench());
  }

  @override
  void dispose() {
    controller.dispose();
    super.dispose();
  }

  Future<void> submit() async {
    final solution = await controller.execute();
    final body = await verify(solution);
    setState(() => result = '${body['code']} (server verify ${body['verify_us']} µs)');
    controller.reset();
  }

  Future<void> bench({int runs = 20}) async {
    final times = <int>[];
    var failures = 0;
    for (var i = 0; i < runs; i++) {
      final token = await fetchChallenge();
      final watch = Stopwatch()..start();
      final done = await solve(token: token, threads: 0).last;
      times.add(watch.elapsedMilliseconds);
      if ((await verify(done.solution!))['code'] != 'ok') failures++;
      setState(() => result = 'bench ${i + 1}/$runs');
    }
    times.sort();
    final summary = 'POWBENCH runs=$runs median=${times[runs ~/ 2 - 1]} p95=${times[(runs * 0.95).ceil() - 1]} '
        'max=${times.last} failures=$failures all=$times';
    debugPrint(summary);
    setState(() => result = summary);
  }

  @override
  Widget build(BuildContext context) => Scaffold(
        appBar: AppBar(title: const Text('nextmoe_pow')),
        body: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            NextmoePow(controller: controller),
            const SizedBox(height: 16),
            Wrap(spacing: 8, children: [
              FilledButton(onPressed: submit, child: const Text('Verify')),
              OutlinedButton(onPressed: bench, child: const Text('Benchmark ×20')),
            ]),
            const SizedBox(height: 16),
            SelectableText(result),
          ],
        ),
      );
}
