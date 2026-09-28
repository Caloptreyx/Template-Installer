import { faArrowLeft, faCircleQuestion } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import Badge from '@/elements/data-display/Badge.tsx';
import Group from '@/elements/layout/Group.tsx';
import Tabs from '@/elements/layout/Tabs.tsx';
import Title from '@/elements/typography/Title.tsx';
import { useToast } from '@/providers/ToastProvider.tsx';
import { useTranslations } from '@/providers/TranslationProvider.tsx';
import {
  createAdminTemplate,
  type Step,
  type Template,
  type TemplatePayload,
  updateAdminTemplate,
  type Variable,
} from '../../../api.ts';
import { stepFieldErrors } from '../../../lib/steps.ts';
import { variableKeyError } from '../../../lib/variables.ts';
import { useExtTranslations } from '../../../translations.ts';
import GeneralTab, { type GeneralValues } from './GeneralTab.tsx';
import { type Keyed, keyed } from './keyedList.ts';
import StepsEditor from './StepsEditor.tsx';
import VariablesEditor from './VariablesEditor.tsx';

type EditorTab = 'general' | 'variables' | 'steps';

export default function TemplateEditor({
  template,
  onClose,
  onHelp,
}: {
  template?: Template;
  onClose: () => void;
  onHelp: () => void;
}) {
  const { t: tExt } = useExtTranslations();
  const { t } = useTranslations();
  const { addToast } = useToast();

  const [tab, setTab] = useState<EditorTab>('general');
  const [general, setGeneral] = useState<GeneralValues>({
    name: template?.name ?? '',
    description: template?.description ?? '',
    enabled: template?.enabled ?? true,
    eggUuids: template?.eggUuids ?? [],
  });
  const [variables, setVariables] = useState<Keyed<Variable>[]>(() => (template?.variables ?? []).map(keyed));
  const [steps, setSteps] = useState<Keyed<Step>[]>(() => (template?.steps ?? []).map(keyed));
  const [showErrors, setShowErrors] = useState(false);
  const [saving, setSaving] = useState(false);

  const variableValues = variables.map((row) => row.value);
  const tabErrors: Record<EditorTab, boolean> = {
    general: !general.name.trim(),
    variables: variableValues.some(
      (variable, index) => variableKeyError(variableValues, index) !== null || !variable.label.trim(),
    ),
    steps: steps.some((row) => Object.keys(stepFieldErrors(row.value)).length > 0),
  };

  const doSave = async () => {
    const invalidTab = (['general', 'variables', 'steps'] as const).find((name) => tabErrors[name]);
    if (invalidTab) {
      setShowErrors(true);
      setTab(invalidTab);
      addToast(tExt('editor.invalid', {}), 'error');
      return;
    }

    const payload: TemplatePayload = {
      name: general.name.trim(),
      description: general.description.trim() ? general.description : null,
      enabled: general.enabled,
      eggUuids: general.eggUuids,
      variables: variableValues.map((variable) => ({
        ...variable,
        label: variable.label.trim(),
        description: variable.description?.trim() ? variable.description : null,
      })),
      steps: steps.map((row) => row.value),
    };

    setSaving(true);
    try {
      if (template) {
        await updateAdminTemplate(template.uuid, payload);
        addToast(tExt('editor.updated', {}), 'success');
      } else {
        await createAdminTemplate(payload);
        addToast(tExt('editor.created', {}), 'success');
      }
      onClose();
    } catch (msg) {
      addToast(httpErrorToHuman(msg), 'error');
      setSaving(false);
    }
  };

  const tabLabel = (name: EditorTab, count?: number) => (
    <Group gap={6} wrap='nowrap'>
      {tExt(`editor.tabs.${name}`, {})}
      {count !== undefined && (
        <Badge size='sm' variant='light' color={showErrors && tabErrors[name] ? 'red' : 'gray'}>
          {count}
        </Badge>
      )}
    </Group>
  );

  return (
    <>
      <Group justify='space-between' mb='md'>
        <Group gap='sm'>
          <Button variant='subtle' color='gray' leftSection={<FontAwesomeIcon icon={faArrowLeft} />} onClick={onClose}>
            {tExt('editor.back', {})}
          </Button>
          <Title order={3}>
            {template ? tExt('editor.editTitle', { name: template.name }) : tExt('editor.createTitle', {})}
          </Title>
        </Group>
        <Group>
          <Button variant='default' leftSection={<FontAwesomeIcon icon={faCircleQuestion} />} onClick={onHelp}>
            {tExt('common.help', {})}
          </Button>
          <Button variant='default' onClick={onClose}>
            {t('common.button.cancel', {})}
          </Button>
          <Button onClick={doSave} loading={saving}>
            {template ? t('common.button.save', {}) : t('common.button.create', {})}
          </Button>
        </Group>
      </Group>

      <Tabs value={tab} onChange={(value) => value && setTab(value as EditorTab)}>
        <Tabs.List>
          <Tabs.Tab value='general' color={showErrors && tabErrors.general ? 'red' : undefined}>
            {tabLabel('general')}
          </Tabs.Tab>
          <Tabs.Tab value='variables' color={showErrors && tabErrors.variables ? 'red' : undefined}>
            {tabLabel('variables', variables.length)}
          </Tabs.Tab>
          <Tabs.Tab value='steps' color={showErrors && tabErrors.steps ? 'red' : undefined}>
            {tabLabel('steps', steps.length)}
          </Tabs.Tab>
        </Tabs.List>

        <Tabs.Panel value='general' pt='md'>
          <GeneralTab
            values={general}
            onChange={(patch) => setGeneral((current) => ({ ...current, ...patch }))}
            showErrors={showErrors}
          />
        </Tabs.Panel>
        <Tabs.Panel value='variables' pt='md'>
          <VariablesEditor value={variables} onChange={setVariables} showErrors={showErrors} />
        </Tabs.Panel>
        <Tabs.Panel value='steps' pt='md'>
          <StepsEditor value={steps} onChange={setSteps} showErrors={showErrors} />
        </Tabs.Panel>
      </Tabs>
    </>
  );
}
