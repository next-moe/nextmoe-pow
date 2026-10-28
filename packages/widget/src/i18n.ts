export type Locale = 'en' | 'zh' | 'ja'

export interface Messages {
  label: string
  verifying: string
  verified: string
  failed: string
  expired: string
  retry: string
}

export const MESSAGES: Record<Locale, Messages> = {
  en: {
    label: 'Bot check',
    verifying: 'Checking your browser…',
    verified: 'Verified',
    failed: 'Verification failed',
    expired: 'Expired, checking again…',
    retry: 'Retry',
  },
  zh: {
    label: '人机验证',
    verifying: '正在验证浏览器…',
    verified: '验证完成',
    failed: '验证失败',
    expired: '已过期，正在重新验证…',
    retry: '重试',
  },
  ja: {
    label: 'ボット対策の確認',
    verifying: 'ブラウザを確認しています…',
    verified: '確認しました',
    failed: '確認に失敗しました',
    expired: '期限切れのため再確認しています…',
    retry: '再試行',
  },
}

export const resolveLocale = (...candidates: Array<string | null | undefined>): Locale => {
  for (const candidate of candidates) {
    const base = candidate?.toLowerCase().split('-')[0]
    if (base && base in MESSAGES) return base as Locale
  }
  return 'en'
}
