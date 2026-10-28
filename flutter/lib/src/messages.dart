import 'dart:ui';

class PowMessages {
  const PowMessages({
    required this.label,
    required this.verifying,
    required this.verified,
    required this.failed,
    required this.expired,
    required this.retry,
  });

  final String label;
  final String verifying;
  final String verified;
  final String failed;
  final String expired;
  final String retry;

  static const en = PowMessages(
    label: 'Bot check',
    verifying: 'Checking your device…',
    verified: 'Verified',
    failed: 'Verification failed',
    expired: 'Expired, checking again…',
    retry: 'Retry',
  );

  static const zh = PowMessages(
    label: '人机验证',
    verifying: '正在验证设备…',
    verified: '验证完成',
    failed: '验证失败',
    expired: '已过期，正在重新验证…',
    retry: '重试',
  );

  static const ja = PowMessages(
    label: 'ボット対策の確認',
    verifying: 'デバイスを確認しています…',
    verified: '確認しました',
    failed: '確認に失敗しました',
    expired: '期限切れのため再確認しています…',
    retry: '再試行',
  );

  static PowMessages of(Locale? locale) => switch (locale?.languageCode) {
        'zh' => zh,
        'ja' => ja,
        _ => en,
      };
}
