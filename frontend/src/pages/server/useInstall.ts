import { useQuery } from '@tanstack/react-query';
import { getServerInstall, templatesQueryKey } from '../../api.ts';

const POLL_INTERVAL_MS = 1500;

export const serverTemplatesQueryKey = (serverUuid: string) => [...templatesQueryKey, 'server', serverUuid] as const;

export const installQueryKey = (serverUuid: string, installUuid: string | null) =>
  [...serverTemplatesQueryKey(serverUuid), 'install', installUuid] as const;

// one install, polled while it is running; every observer of the same install shares the cache entry
export default function useInstall(serverUuid: string, installUuid: string | null) {
  return useQuery({
    queryKey: installQueryKey(serverUuid, installUuid),
    queryFn: () => getServerInstall(serverUuid, installUuid ?? ''),
    enabled: installUuid !== null,
    refetchInterval: (query) => (query.state.data?.status === 'running' ? POLL_INTERVAL_MS : false),
  });
}
