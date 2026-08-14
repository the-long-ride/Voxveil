export const SUPPORTED_LANGUAGES = ['en', 'vi', 'zh', 'ko', 'ja', 'es', 'fr'] as const;
export type SupportedLanguage = typeof SUPPORTED_LANGUAGES[number];
