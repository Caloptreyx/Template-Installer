import { Center } from '@mantine/core';
import type { UseQueryResult } from '@tanstack/react-query';
import { httpErrorToHuman } from '@/api/axios.ts';
import Table, { TableData, TableRow } from '@/elements/data-display/Table.tsx';
import FormattedTimestamp from '@/elements/time/FormattedTimestamp.tsx';
import Text from '@/elements/typography/Text.tsx';
import type { Install } from '../../api.ts';
import { InstallStatusBadge } from '../../components/StatusBadges.tsx';
import { useExtTranslations } from '../../translations.ts';

export default function RecentInstalls({
  query,
  onSelect,
}: {
  query: UseQueryResult<Install[]>;
  onSelect: (install: Install) => void;
}) {
  const { t: tExt } = useExtTranslations();
  const { data, isLoading, error } = query;

  const count = data?.length ?? 0;

  return (
    <Table
      columns={[
        tExt('table.template', {}),
        tExt('table.user', {}),
        tExt('table.status', {}),
        tExt('table.started', {}),
      ]}
      loading={isLoading}
      error={error ? httpErrorToHuman(error) : null}
      pagination={{ total: count, perPage: Math.max(count, 1), page: 1, data: data ?? [] }}
      empty={
        <Center py='lg'>
          <Text c='dimmed'>{tExt('pages.server.installsEmpty', {})}</Text>
        </Center>
      }
    >
      {data?.map((install) => (
        <TableRow key={install.uuid} className='cursor-pointer' onClick={() => onSelect(install)}>
          <TableData>
            <Text size='sm' fw={500}>
              {install.templateName}
            </Text>
          </TableData>
          <TableData>{install.user?.username ?? tExt('install.unknownUser', {})}</TableData>
          <TableData>
            <InstallStatusBadge status={install.status} />
          </TableData>
          <TableData>
            <FormattedTimestamp timestamp={install.created} />
          </TableData>
        </TableRow>
      ))}
    </Table>
  );
}
