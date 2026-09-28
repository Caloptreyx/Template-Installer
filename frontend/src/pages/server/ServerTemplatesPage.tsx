import { faWandMagicSparkles } from '@fortawesome/free-solid-svg-icons';
import { SimpleGrid } from '@mantine/core';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import ServerContentContainer from '@/elements/containers/ServerContentContainer.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import EmptyState from '@/elements/feedback/EmptyState.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import Title from '@/elements/typography/Title.tsx';
import { useToast } from '@/providers/ToastProvider.tsx';
import { useServerStore } from '@/stores/server.ts';
import { getServerInstalls, getServerTemplates, type Install } from '../../api.ts';
import { useExtTranslations } from '../../translations.ts';
import InstallModal, { type InstallDialog } from './InstallModal.tsx';
import RecentInstalls from './RecentInstalls.tsx';
import TemplateCard from './TemplateCard.tsx';
import useInstall, { installQueryKey, serverTemplatesQueryKey } from './useInstall.ts';

const RUNNING_INSTALLS_POLL_MS = 5000;

export default function ServerTemplatesPage() {
  const { t: tExt } = useExtTranslations();
  const { addToast } = useToast();
  const queryClient = useQueryClient();
  const server = useServerStore((state) => state.server);

  const [search, setSearch] = useState('');
  const [dialog, setDialog] = useState<InstallDialog | null>(null);
  // the install started from this page; it is followed until it finishes, even with the dialog closed
  const [watchedUuid, setWatchedUuid] = useState<string | null>(null);

  const baseKey = serverTemplatesQueryKey(server.uuid);
  const templates = useQuery({
    queryKey: [...baseKey, 'templates'],
    queryFn: () => getServerTemplates(server.uuid),
  });
  const installs = useQuery({
    queryKey: [...baseKey, 'installs'],
    queryFn: () => getServerInstalls(server.uuid),
    refetchInterval: (query) =>
      query.state.data?.some((install) => install.status === 'running') ? RUNNING_INSTALLS_POLL_MS : false,
  });
  const watched = useInstall(server.uuid, watchedUuid);

  useEffect(() => {
    const install = watched.data;
    if (!install || install.status === 'running') return;

    if (install.status === 'success') {
      addToast(tExt('install.succeeded', { name: install.templateName }), 'success');
    } else {
      addToast(tExt('install.failed', { name: install.templateName }), 'error');
    }
    setWatchedUuid(null);
    installs.refetch();
  }, [watched.data?.status]);

  const onStarted = (install: Install) => {
    queryClient.setQueryData(installQueryKey(server.uuid, install.uuid), install);
    setWatchedUuid(install.uuid);
    setDialog({ kind: 'progress', installUuid: install.uuid, templateName: install.templateName });
    installs.refetch();
  };

  const query = search.trim().toLowerCase();
  const filtered = (templates.data ?? []).filter(
    (template) =>
      !query ||
      template.name.toLowerCase().includes(query) ||
      (template.description ?? '').toLowerCase().includes(query),
  );

  return (
    <ServerContentContainer title={tExt('common.templates', {})} search={search} setSearch={setSearch}>
      <InstallModal serverUuid={server.uuid} dialog={dialog} onStarted={onStarted} onClose={() => setDialog(null)} />

      {templates.isLoading ? (
        <Spinner.Centered />
      ) : templates.error ? (
        <Alert color='red'>{httpErrorToHuman(templates.error)}</Alert>
      ) : filtered.length === 0 ? (
        <EmptyState
          icon={faWandMagicSparkles}
          title={tExt('pages.server.emptyTitle', {})}
          description={query ? tExt('pages.server.noResults', {}) : tExt('pages.server.empty', {})}
        />
      ) : (
        <SimpleGrid cols={{ base: 1, md: 2, xl: 3 }}>
          {filtered.map((template) => (
            <TemplateCard
              key={template.uuid}
              template={template}
              onInstall={() => setDialog({ kind: 'form', template })}
            />
          ))}
        </SimpleGrid>
      )}

      <Title order={3} mt='xl' mb='md'>
        {tExt('pages.server.recentInstalls', {})}
      </Title>
      <RecentInstalls
        query={installs}
        onSelect={(install) =>
          setDialog({ kind: 'progress', installUuid: install.uuid, templateName: install.templateName })
        }
      />
    </ServerContentContainer>
  );
}
