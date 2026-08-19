import type { SystemAudioEndpoint } from '../../lib/types';

export interface SystemAudioEndpointsProps {
  endpoints: SystemAudioEndpoint[];
  busy: boolean;
  installBusyId: string | null;
  error: string | null;
  onRefresh: () => void;
  onInstall: (endpointId: string) => void;
  onInstallAll: () => void;
}

export function SystemAudioEndpoints(_props: SystemAudioEndpointsProps) {
  return null;
}
