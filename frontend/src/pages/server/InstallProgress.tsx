import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { SimpleGrid } from '@mantine/core';
import type { ReactNode } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Alert from '@/elements/feedback/Alert.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import Group from '@/elements/layout/Group.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import Timeline from '@/elements/layout/Timeline.tsx';
import FormattedTimestamp from '@/elements/time/FormattedTimestamp.tsx';
import Text from '@/elements/typography/Text.tsx';
import { InstallStatusBadge, STEP_STATUS_COLORS, StepStatusIcon } from '../../components/StatusBadges.tsx';
import { STEP_ICONS } from '../../lib/steps.ts';
import { useExtTranslations } from '../../translations.ts';
import useInstall from './useInstall.ts';

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div>
      <Text size='xs' c='dimmed'>
        {label}
      </Text>
      <div>{children}</div>
    </div>
  );
}

export default function InstallProgress({ serverUuid, installUuid }: { serverUuid: string; installUuid: string }) {
  const { t: tExt } = useExtTranslations();
  const { data: install, isLoading, error } = useInstall(serverUuid, installUuid);

  if (isLoading) return <Spinner.Centered />;
  if (error || !install) return <Alert color='red'>{httpErrorToHuman(error)}</Alert>;

  const active = install.steps.reduce((last, step, index) => (step.status === 'pending' ? last : index), -1);

  return (
    <Stack gap='md'>
      <SimpleGrid cols={{ base: 2, sm: 4 }} spacing='md'>
        <Field label={tExt('table.status', {})}>
          <InstallStatusBadge status={install.status} />
        </Field>
        <Field label={tExt('install.by', {})}>
          <Text size='sm'>{install.user?.username ?? tExt('install.unknownUser', {})}</Text>
        </Field>
        <Field label={tExt('install.startedAt', {})}>
          <Text size='sm'>
            <FormattedTimestamp timestamp={install.created} />
          </Text>
        </Field>
        <Field label={tExt('install.finishedAt', {})}>
          <Text size='sm'>
            <FormattedTimestamp timestamp={install.finished} showNA />
          </Text>
        </Field>
      </SimpleGrid>

      {install.error && (
        <Alert color='red' title={tExt('install.error', {})}>
          <Text size='sm' className='whitespace-pre-wrap break-all'>
            {install.error}
          </Text>
        </Alert>
      )}

      <Timeline active={active} bulletSize={24} lineWidth={2}>
        {install.steps.map((step) => (
          <Timeline.Item
            key={step.index}
            color={STEP_STATUS_COLORS[step.status]}
            bullet={<StepStatusIcon status={step.status} />}
            title={
              <Group gap='xs' wrap='nowrap'>
                <FontAwesomeIcon icon={STEP_ICONS[step.type]} size='sm' fixedWidth />
                <Text size='sm' fw={500} c={step.status === 'skipped' ? 'dimmed' : undefined}>
                  {step.name || tExt(`steps.types.${step.type}.label`, {})}
                </Text>
              </Group>
            }
          >
            {step.message && (
              <Text size='xs' c={step.status === 'failed' ? 'red' : 'dimmed'} className='whitespace-pre-wrap break-all'>
                {step.message}
              </Text>
            )}
          </Timeline.Item>
        ))}
      </Timeline>
    </Stack>
  );
}
