import ConfirmationModal from '@/elements/modals/ConfirmationModal.tsx';
import { useToast } from '@/providers/ToastProvider.tsx';
import { deleteAdminTemplate, type Template } from '../../api.ts';
import { useExtTranslations } from '../../translations.ts';

export default function TemplateDeleteModal({
  template,
  onClose,
  onDeleted,
}: {
  template: Template | null;
  onClose: () => void;
  onDeleted: () => void;
}) {
  const { t: tExt } = useExtTranslations();
  const { addToast } = useToast();

  const doDelete = async () => {
    if (!template) return;
    await deleteAdminTemplate(template.uuid);
    addToast(tExt('pages.admin.deleted', {}), 'success');
    onClose();
    onDeleted();
  };

  return (
    <ConfirmationModal
      opened={!!template}
      onClose={onClose}
      title={tExt('pages.admin.deleteTitle', {})}
      confirm={tExt('common.delete', {})}
      onConfirmed={doDelete}
    >
      {tExt('pages.admin.deleteContent', { name: template?.name ?? '' }).md()}
    </ConfirmationModal>
  );
}
