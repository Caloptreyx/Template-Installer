import { useQuery } from '@tanstack/react-query';
import { getAdminEggs, templatesQueryKey } from '../../api.ts';

export default function useAdminEggs() {
  return useQuery({
    queryKey: [...templatesQueryKey, 'admin', 'eggs'],
    queryFn: getAdminEggs,
    staleTime: 60_000,
  });
}
