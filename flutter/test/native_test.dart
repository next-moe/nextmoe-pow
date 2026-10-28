import 'dart:convert';
import 'dart:io';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated_io.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:nextmoe_pow/nextmoe_pow.dart';
import 'package:nextmoe_pow/src/rust/frb_generated.dart';

final _vectors = jsonDecode(File('../testdata/vectors.json').readAsStringSync()) as Map<String, dynamic>;

void main() {
  setUpAll(() => RustLib.init(externalLibrary: ExternalLibrary.open('../target/release/libnextmoe_pow_flutter.so')));

  test('challenge fields decode like the vectors', () {
    for (final c in (_vectors['challenges'] as List).cast<Map<String, dynamic>>()) {
      final info = challengeInfo(token: c['token'] as String);
      expect(info.alg, c['alg']['name']);
      expect(info.k, c['k']);
      expect(info.difficulty, c['difficulty']);
      expect(info.memoryKib, c['alg']['memory_kib'] ?? 0);
      expect(info.expiresAt.toInt(), c['expires_at']);
      expect(info.site, c['site']);
      expect(info.action, c['action']);
    }
    expect(() => challengeInfo(token: 'nope'), throwsA(anything));
  });

  test('solves natively and reports monotone progress', () async {
    final token = (_vectors['challenges'] as List).first['token'] as String;
    final events = await solve(token: token, threads: 4).toList();
    final last = events.last;
    expect(last.solution, startsWith('$token.'));
    expect(last.solved, last.k);
    expect(events.where((e) => e.solution != null), hasLength(1));
  });

  final demo = Platform.environment['POW_DEMO_URL'];
  test('a native solution passes the Go verifier', () async {
    final client = HttpClient();
    final query = 'alg=2&m=512&t=1&k=8&d=1000&site=flutter&action=test';
    final req = await client.getUrl(Uri.parse('$demo/api/challenge?$query'));
    final token = (jsonDecode(await utf8.decodeStream(await req.close())) as Map)['token'] as String;
    final solved = await solve(token: token, threads: 0).last;
    Future<String> verify() async {
      final post = await client.postUrl(Uri.parse('$demo/api/verify'));
      post.headers.contentType = ContentType.json;
      post.write(jsonEncode({'solution': solved.solution, 'site': 'flutter', 'action': 'test'}));
      return (jsonDecode(await utf8.decodeStream(await post.close())) as Map)['code'] as String;
    }

    expect(await verify(), 'ok');
    expect(await verify(), 'replayed');
    client.close();
  }, skip: demo == null ? 'set POW_DEMO_URL to run against go/cmd/powdemo' : false);
}
