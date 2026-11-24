import { useTranslation } from 'react-i18next';
import type { SystemAiStatus } from '../../lib/types';

export function SystemAiNotice({ status }: { status: SystemAiStatus }) {
  const { t } = useTranslation();
  if (status === 'off') return null;
  return <div className="backend-notice" role="status">{t(`aiRelay.${status}`)}</div>;
}
