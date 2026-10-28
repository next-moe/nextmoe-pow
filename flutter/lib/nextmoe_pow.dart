import 'src/rust/frb_generated.dart';

export 'src/controller.dart' show ChallengeFetcher, PowController, PowSolver, PowState;
export 'src/messages.dart';
export 'src/rust/api.dart' show ChallengeInfo, SolveEvent, challengeInfo, solve;
export 'src/style.dart';
export 'src/widget.dart';

/// Loads the native solver; call once before the first [PowController] solves.
Future<void> initNextmoePow() => RustLib.init();
