import { Input } from '@mantine/core';
import MonacoEditor from '@/elements/editors/MonacoEditor.tsx';
import Select from '@/elements/input/Select.tsx';
import Switch from '@/elements/input/Switch.tsx';
import TextArea from '@/elements/input/TextArea.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import { POWER_ACTIONS, type PowerAction, type Step } from '../../../api.ts';
import {
  editorLanguage,
  getStepValue,
  type StepFieldDef,
  type StepFieldError,
  setStepValue,
} from '../../../lib/steps.ts';
import { useExtTranslations } from '../../../translations.ts';

const MONOSPACE = { input: { fontFamily: 'var(--mantine-font-family-monospace)' } };

export default function StepField({
  step,
  field,
  error,
  onChange,
}: {
  step: Step;
  field: StepFieldDef;
  error: StepFieldError | undefined;
  onChange: (step: Step) => void;
}) {
  const { t: tExt } = useExtTranslations();

  const common = {
    label: tExt(`steps.fields.${field.key}`, {}),
    description: tExt(`steps.fieldDescriptions.${field.key}`, {}),
    withAsterisk: field.required,
    error: error ? tExt(`editor.${error}`, {}) : undefined,
  };
  const value = getStepValue(step, field.key);
  const set = (next: unknown) => onChange(setStepValue(step, field.key, next));

  switch (field.kind) {
    case 'boolean':
      return (
        <Switch
          label={common.label}
          description={common.description}
          checked={value === true}
          onChange={(e) => set(e.target.checked)}
        />
      );
    case 'powerAction':
      return (
        <Select
          {...common}
          data={POWER_ACTIONS.map((action) => ({ value: action, label: tExt(`powerActions.${action}`, {}) }))}
          value={value as PowerAction}
          onChange={(action) => action && set(action)}
        />
      );
    case 'code':
      return (
        <Input.Wrapper {...common}>
          <div
            className='mt-1 overflow-hidden rounded-sm'
            style={{ border: '1px solid var(--mantine-color-default-border)' }}
          >
            <MonacoEditor
              height='260px'
              language={editorLanguage(String(getStepValue(step, 'path') ?? ''))}
              value={String(value ?? '')}
              onChange={(content) => set(content ?? '')}
              options={{
                automaticLayout: true,
                minimap: { enabled: false },
                stickyScroll: { enabled: false },
                scrollBeyondLastLine: false,
                wordWrap: 'on',
                tabSize: 2,
              }}
            />
          </div>
        </Input.Wrapper>
      );
    case 'multiline':
      return (
        <TextArea
          {...common}
          value={String(value ?? '')}
          onChange={(e) => set(e.target.value)}
          autosize
          minRows={2}
          maxRows={12}
          spellCheck={false}
          styles={MONOSPACE}
        />
      );
    default:
      return (
        <TextInput
          {...common}
          value={String(value ?? '')}
          onChange={(e) => set(e.target.value)}
          spellCheck={false}
          styles={MONOSPACE}
        />
      );
  }
}
