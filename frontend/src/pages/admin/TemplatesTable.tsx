import {
  faCircleQuestion,
  faFileExport,
  faFileImport,
  faPen,
  faPlus,
  faSearch,
  faTrash,
} from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Center, FileButton } from '@mantine/core';
import { useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import { AdminCan } from '@/elements/Can.tsx';
import Badge from '@/elements/data-display/Badge.tsx';
import Table, { TableData, TableRow } from '@/elements/data-display/Table.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import Group from '@/elements/layout/Group.tsx';
import ContextMenu, { ContextMenuToggle } from '@/elements/overlays/ContextMenu.tsx';
import Tooltip from '@/elements/overlays/Tooltip.tsx';
import FormattedTimestamp from '@/elements/time/FormattedTimestamp.tsx';
import Text from '@/elements/typography/Text.tsx';
import { downloadTextFile } from '@/lib/download/download.ts';
import { useSearchablePaginatedTable } from '@/plugins/resource/useSearchablePaginatedTable.ts';
import { useAdminCan } from '@/plugins/usePermissions.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import { useTranslations } from '@/providers/TranslationProvider.tsx';
import {
  exportAdminTemplate,
  getAdminTemplates,
  importAdminTemplate,
  type Template,
  templatesQueryKey,
} from '../../api.ts';
import { useExtTranslations } from '../../translations.ts';
import TemplateDeleteModal from './TemplateDeleteModal.tsx';
import useAdminEggs from './useAdminEggs.ts';

function exportFileName(name: string): string {
  const slug = name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
  return `${slug || 'template'}.template.json`;
}

export default function TemplatesTable({
  onCreate,
  onEdit,
  onHelp,
}: {
  onCreate: () => void;
  onEdit: (template: Template) => void;
  onHelp: () => void;
}) {
  const { t: tExt, tItem } = useExtTranslations();
  const { t } = useTranslations();
  const { addToast } = useToast();
  const canManage = useAdminCan('templates.manage');
  const { data: eggs } = useAdminEggs();

  const [deleting, setDeleting] = useState<Template | null>(null);
  const [importing, setImporting] = useState(false);

  const { data, loading, error, search, setSearch, setPage, refetch } = useSearchablePaginatedTable({
    queryKey: [...templatesQueryKey, 'admin', 'templates'],
    fetcher: getAdminTemplates,
  });

  const eggNames = (template: Template) =>
    template.eggUuids.map((uuid) => {
      const egg = eggs?.find((egg) => egg.uuid === uuid);
      return egg ? `${egg.nest.name} / ${egg.name}` : uuid;
    });

  const doExport = async (template: Template) => {
    try {
      downloadTextFile(await exportAdminTemplate(template.uuid), exportFileName(template.name), 'application/json');
      addToast(tExt('pages.admin.exported', {}), 'success');
    } catch (msg) {
      addToast(httpErrorToHuman(msg), 'error');
    }
  };

  const doImport = async (file: File | null) => {
    if (!file) return;

    const text = await file.text();
    try {
      JSON.parse(text);
    } catch {
      addToast(tExt('pages.admin.importInvalid', {}), 'error');
      return;
    }

    setImporting(true);
    try {
      const template = await importAdminTemplate(text);
      addToast(tExt('pages.admin.imported', { name: template.name }), 'success');
      refetch();
    } catch (msg) {
      addToast(httpErrorToHuman(msg), 'error');
    }
    setImporting(false);
  };

  return (
    <>
      <TemplateDeleteModal template={deleting} onClose={() => setDeleting(null)} onDeleted={refetch} />

      <Group justify='space-between' mb='md'>
        <TextInput
          placeholder={tExt('pages.admin.search', {})}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          leftSection={<FontAwesomeIcon icon={faSearch} />}
          w={260}
        />
        <Group>
          <Button variant='default' leftSection={<FontAwesomeIcon icon={faCircleQuestion} />} onClick={onHelp}>
            {tExt('common.help', {})}
          </Button>
          <AdminCan action='templates.manage'>
            <FileButton onChange={doImport} accept='application/json,.json'>
              {(props) => (
                <Button
                  {...props}
                  variant='default'
                  loading={importing}
                  leftSection={<FontAwesomeIcon icon={faFileImport} />}
                >
                  {t('common.button.import', {})}
                </Button>
              )}
            </FileButton>
            <Button onClick={onCreate} color='blue' leftSection={<FontAwesomeIcon icon={faPlus} />}>
              {tExt('pages.admin.create', {})}
            </Button>
          </AdminCan>
        </Group>
      </Group>

      <Table
        columns={[
          tExt('table.name', {}),
          tExt('table.enabled', {}),
          tExt('table.eggs', {}),
          tExt('table.steps', {}),
          tExt('table.updated', {}),
          '',
        ]}
        loading={loading}
        error={error}
        pagination={data}
        onPageSelect={setPage}
        empty={
          <Center py='lg'>
            <Text c='dimmed'>{tExt('pages.admin.empty', {})}</Text>
          </Center>
        }
      >
        {data?.data.map((template) => (
          <ContextMenu
            key={template.uuid}
            items={[
              {
                type: 'action',
                icon: faPen,
                label: tExt('common.edit', {}),
                onClick: () => onEdit(template),
                color: 'gray',
                canAccess: canManage,
              },
              {
                type: 'action',
                icon: faFileExport,
                label: tExt('common.export', {}),
                onClick: () => doExport(template),
                color: 'gray',
              },
              {
                type: 'action',
                icon: faTrash,
                label: tExt('common.delete', {}),
                onClick: () => setDeleting(template),
                color: 'red',
                canAccess: canManage,
              },
            ]}
          >
            {({ items, openMenu }) => (
              <TableRow
                className={canManage ? 'cursor-pointer' : undefined}
                onClick={canManage ? () => onEdit(template) : undefined}
                onContextMenu={(e) => {
                  e.preventDefault();
                  openMenu(e.clientX, e.clientY);
                }}
              >
                <TableData>
                  <Text size='sm' fw={500}>
                    {template.name}
                  </Text>
                  {template.description && (
                    <Text size='xs' c='dimmed' className='max-w-96 truncate'>
                      {template.description}
                    </Text>
                  )}
                </TableData>
                <TableData>
                  <Badge color={template.enabled ? 'green' : 'gray'}>
                    {t(template.enabled ? 'common.yes' : 'common.no', {})}
                  </Badge>
                </TableData>
                <TableData>
                  {template.eggUuids.length === 0 ? (
                    <Badge color='blue' variant='light'>
                      {tExt('pages.admin.allEggs', {})}
                    </Badge>
                  ) : (
                    <Tooltip label={eggNames(template).join(', ')} multiline maw={400}>
                      <Text size='sm'>{tItem('egg', template.eggUuids.length)}</Text>
                    </Tooltip>
                  )}
                </TableData>
                <TableData>{template.steps.length}</TableData>
                <TableData>
                  <FormattedTimestamp timestamp={template.updated} />
                </TableData>
                <ContextMenuToggle items={items} openMenu={openMenu} />
              </TableRow>
            )}
          </ContextMenu>
        ))}
      </Table>
    </>
  );
}
