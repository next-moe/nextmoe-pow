import 'dart:async';

import 'package:flutter/foundation.dart';

import 'rust/api.dart' as rust;

enum PowState { idle, verifying, solved, error }

typedef ChallengeFetcher = Future<String> Function();
typedef PowSolver = Stream<rust.SolveEvent> Function(String token);

const _expiryMargin = Duration(seconds: 10);
const _minRefresh = Duration(seconds: 30);

/// When to fetch a fresh challenge; a skewed device clock can delay it but never loop it.
Duration refreshDelay(int expiresAtSeconds, DateTime now) {
  final remaining = DateTime.fromMillisecondsSinceEpoch(expiresAtSeconds * 1000).difference(now) - _expiryMargin;
  return remaining > _minRefresh ? remaining : _minRefresh;
}

double progressFraction(rust.SolveEvent e) {
  final bySolved = e.solved / e.k;
  final byAttempts = (e.attempts / e.expectedAttempts).clamp(0.0, 0.95);
  return bySolved > byAttempts ? bySolved : byAttempts;
}

/// Fetches a challenge, solves it natively and keeps the solution fresh until it is used.
/// Without a [NextmoePow] widget on screen it is the invisible mode: call [execute].
class PowController extends ChangeNotifier {
  PowController({required this.fetchChallenge, PowSolver? solver, this.threads = 0})
      : _solver = solver ?? ((token) => rust.solve(token: token, threads: threads));

  final ChallengeFetcher fetchChallenge;
  final int threads;
  final PowSolver _solver;

  PowState _state = PowState.idle;
  double _progress = 0;
  String? _solution;
  bool _renewing = false;
  Object? _error;
  Completer<String>? _run;
  bool _inFlight = false;
  StreamSubscription<rust.SolveEvent>? _subscription;
  Timer? _refresh;
  int _generation = 0;
  bool _disposed = false;

  PowState get state => _state;
  double get progress => _progress;
  String? get solution => _solution;
  bool get renewing => _renewing;
  Object? get error => _error;

  /// Completes with the solution, starting a solve if none is running. A [reset]
  /// while waiting does not fail the caller: it gets the next solution instead.
  Future<String> execute() {
    final solution = _solution;
    if (solution != null) return Future.value(solution);
    final run = _run ??= Completer<String>();
    if (!_inFlight) {
      _inFlight = true;
      _start(++_generation);
    }
    return run.future;
  }

  /// Drops the solution (for example after it was submitted) and solves a new challenge.
  void reset() {
    _stop();
    _solution = null;
    _renewing = false;
    _set(PowState.idle, 0);
    execute().ignore();
  }

  Future<void> _start(int generation) async {
    _error = null;
    _set(PowState.verifying, 0);
    try {
      final token = await fetchChallenge();
      if (generation != _generation) return;
      _subscription = _solver(token).listen(
        (event) {
          final solution = event.solution;
          if (solution == null) return _set(PowState.verifying, progressFraction(event));
          _inFlight = false;
          _solution = solution;
          _renewing = false;
          _set(PowState.solved, 1);
          _refresh = Timer(refreshDelay(event.expiresAt.toInt(), DateTime.now()), _expire);
          _finish((run) => run.complete(solution));
        },
        onError: (Object error) => _fail(generation, error),
        cancelOnError: true,
      );
    } catch (error) {
      _fail(generation, error);
    }
  }

  void _fail(int generation, Object error) {
    if (generation != _generation) return;
    _inFlight = false;
    _error = error;
    _set(PowState.error, _progress);
    _finish((run) => run.completeError(error));
  }

  void _finish(void Function(Completer<String> run) settle) {
    final run = _run;
    _run = null;
    if (run != null) settle(run);
  }

  void _expire() {
    _solution = null;
    _renewing = true;
    execute().ignore();
  }

  void _stop() {
    _generation++;
    _inFlight = false;
    _subscription?.cancel();
    _subscription = null;
    _refresh?.cancel();
  }

  void _set(PowState state, double progress) {
    if (_disposed) return;
    _state = state;
    _progress = progress;
    notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    _stop();
    _finish((run) => run.completeError(StateError('PowController disposed')));
    super.dispose();
  }
}
