import { faWandMagicSparkles } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { type FormEvent, useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import { Modal, ModalFooter } from '@/elements/modals/Modal.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useToast } from '@/providers/ToastProvider.tsx';
import { useTranslations } from '@/providers/TranslationProvider.tsx';
import { type ClientTemplate, type Install, installServerTemplate } from '../../api.ts';
import VariableInput from '../../components/VariableInput.tsx';
import { initialVariableValue, isRequired } from '../../lib/variables.ts';
import { useExtTranslations } from '../../translations.ts';
import InstallProgress from './InstallProgress.tsx';

export type InstallDialog =
  | { kind: 'form'; template: ClientTemplate }
  | { kind: 'progress'; installUuid: string; templateName: string };

function InstallForm({
  serverUuid,
  template,
  onStarted,
  onCancel,
}: {
  serverUuid: string;
  template: ClientTemplate;
  onStarted: (install: Install) => void;
  onCancel: () => void;
}) {
  const { t: tExt } = useExtTranslations();
  const { t } = useTranslations();
  const { addToast } = useToast();

  const [values, setValues] = useState<Record<string, string>>(() =>
    Object.fromEntries(template.variables.map((variable) => [variable.key, initialVariableValue(variable)])),
  );
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(false);

  const doSubmit = async (e: FormEvent) => {
    e.preventDefault();

    const missing = Object.fromEntries(
      template.variables
        .filter((variable) => isRequired(variable) && !(values[variable.key] ?? '').trim())
        .map((variable) => [variable.key, tExt('install.required', {})]),
    );
    setErrors(missing);
    if (Object.keys(missing).length > 0) return;

    setLoading(true);
    try {
      const install = await installServerTemplate(serverUuid, template.uuid, {
        variables: template.variables.map((variable) => ({ key: variable.key, value: values[variable.key] ?? '' })),
      });
      addToast(tExt('install.started', { name: template.name }), 'success');
      onStarted(install);
    } catch (error) {
      const message = httpErrorToHuman(error);
      // validation errors read "KEY: reason"; show them on the matching input
      const variable = template.variables.find((variable) => message.startsWith(`${variable.key}:`));
      if (variable) {
        setErrors({ [variable.key]: message.slice(variable.key.length + 1).trim() });
      } else {
        addToast(message, 'error');
      }
    }
    setLoading(false);
  };

  return (
    <form onSubmit={doSubmit}>
      <Stack gap='md'>
        {template.description && (
          <Text size='sm' c='dimmed' className='whitespace-pre-wrap'>
            {template.description}
          </Text>
        )}
        <Alert color='yellow'>{tExt('install.warning', {})}</Alert>

        {template.variables.length === 0 ? (
          <Text size='sm'>{tExt('install.noVariables', {})}</Text>
        ) : (
          template.variables.map((variable) => (
            <VariableInput
              key={variable.key}
              variable={variable}
              value={values[variable.key] ?? ''}
              onChange={(value) => setValues((current) => ({ ...current, [variable.key]: value }))}
              error={errors[variable.key]}
            />
          ))
        )}

        <ModalFooter>
          <Button type='submit' loading={loading} leftSection={<FontAwesomeIcon icon={faWandMagicSparkles} />}>
            {tExt('install.install', {})}
          </Button>
          <Button variant='default' onClick={onCancel}>
            {t('common.button.cancel', {})}
          </Button>
        </ModalFooter>
      </Stack>
    </form>
  );
}

export default function InstallModal({
  serverUuid,
  dialog,
  onStarted,
  onClose,
}: {
  serverUuid: string;
  dialog: InstallDialog | null;
  onStarted: (install: Install) => void;
  onClose: () => void;
}) {
  const { t: tExt } = useExtTranslations();
  const { t } = useTranslations();

  return (
    <Modal
      size='lg'
      opened={dialog !== null}
      onClose={onClose}
      title={
        dialog?.kind === 'form'
          ? tExt('install.title', { name: dialog.template.name })
          : dialog?.kind === 'progress'
            ? dialog.templateName
            : undefined
      }
    >
      {dialog?.kind === 'form' && (
        <InstallForm
          key={dialog.template.uuid}
          serverUuid={serverUuid}
          template={dialog.template}
          onStarted={onStarted}
          onCancel={onClose}
        />
      )}
      {dialog?.kind === 'progress' && (
        <>
          <InstallProgress serverUuid={serverUuid} installUuid={dialog.installUuid} />
          <ModalFooter>
            <Button variant='default' onClick={onClose}>
              {t('common.button.close', {})}
            </Button>
          </ModalFooter>
        </>
      )}
    </Modal>
  );
}
