import { faChevronDown, faTriangleExclamation } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { UnstyledButton } from '@mantine/core';
import { useState } from 'react';
import Button from '@/elements/buttons/Button.tsx';
import Card from '@/elements/data-display/Card.tsx';
import ThemeIcon from '@/elements/data-display/ThemeIcon.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import Collapse from '@/elements/layout/Collapse.tsx';
import Group from '@/elements/layout/Group.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import Text from '@/elements/typography/Text.tsx';
import { MAX_LABEL, type Step } from '../../../api.ts';
import { STEP_FIELDS, STEP_ICONS, type StepFieldDef, type StepFieldError, stepSummary } from '../../../lib/steps.ts';
import { useExtTranslations } from '../../../translations.ts';
import ItemActions from './ItemActions.tsx';
import StepField from './StepField.tsx';

const MONOSPACE = { input: { fontFamily: 'var(--mantine-font-family-monospace)' } };

function Chevron({ open }: { open: boolean }) {
  return (
    <FontAwesomeIcon
      icon={faChevronDown}
      size='xs'
      style={{ transform: open ? 'rotate(0deg)' : 'rotate(-90deg)', transition: 'transform 0.15s ease' }}
    />
  );
}

export default function StepCard({
  step,
  index,
  count,
  errors,
  showErrors,
  initiallyExpanded,
  canDuplicate,
  onChange,
  onMove,
  onDuplicate,
  onRemove,
}: {
  step: Step;
  index: number;
  count: number;
  errors: Record<string, StepFieldError>;
  showErrors: boolean;
  initiallyExpanded: boolean;
  canDuplicate: boolean;
  onChange: (step: Step) => void;
  onMove: (offset: -1 | 1) => void;
  onDuplicate: () => void;
  onRemove: () => void;
}) {
  const { t: tExt } = useExtTranslations();

  const [expanded, setExpanded] = useState(initiallyExpanded);
  const [advanced, setAdvanced] = useState(!!step.condition);

  const fields: readonly StepFieldDef[] = STEP_FIELDS[step.type];
  // required-field errors appear once saving was attempted; size errors right away
  const visibleErrors = Object.fromEntries(
    Object.entries(errors).filter(([, error]) => showErrors || error !== 'fieldRequired'),
  );
  const hasErrors = Object.keys(visibleErrors).length > 0;
  const summary = step.name || stepSummary(step);

  return (
    <Card p='sm' style={hasErrors ? { borderColor: 'var(--mantine-color-red-6)' } : undefined}>
      <Group justify='space-between' wrap='nowrap' gap='sm'>
        <UnstyledButton onClick={() => setExpanded(!expanded)} className='min-w-0 flex-1'>
          <Group gap='sm' wrap='nowrap'>
            <Chevron open={expanded} />
            <ThemeIcon variant='light' size='md'>
              <FontAwesomeIcon icon={STEP_ICONS[step.type]} size='sm' />
            </ThemeIcon>
            <div className='min-w-0'>
              <Group gap='xs' wrap='nowrap'>
                <Text size='sm' fw={500} className='shrink-0'>
                  {tExt('editor.stepNumber', { number: index + 1 })} · {tExt(`steps.types.${step.type}.label`, {})}
                </Text>
                {hasErrors && (
                  <FontAwesomeIcon icon={faTriangleExclamation} size='xs' color='var(--mantine-color-red-6)' />
                )}
              </Group>
              {summary && (
                <Text size='xs' c='dimmed' ff={step.name ? undefined : 'monospace'} className='truncate'>
                  {summary}
                </Text>
              )}
            </div>
          </Group>
        </UnstyledButton>
        <ItemActions
          index={index}
          count={count}
          onMove={onMove}
          onDuplicate={onDuplicate}
          duplicateDisabled={!canDuplicate}
          onRemove={onRemove}
        />
      </Group>

      {expanded && (
        <Stack gap='sm' pt='sm'>
          <Text size='xs' c='dimmed'>
            {tExt(`steps.types.${step.type}.description`, {})}
          </Text>
          <TextInput
            label={tExt('editor.stepName', {})}
            description={tExt('editor.stepNameDescription', {})}
            value={step.name ?? ''}
            onChange={(e) => onChange({ ...step, name: e.target.value || null })}
            maxLength={MAX_LABEL}
          />
          {fields.map((field) => (
            <StepField key={field.key} step={step} field={field} error={visibleErrors[field.key]} onChange={onChange} />
          ))}

          <div>
            <Button
              variant='subtle'
              size='xs'
              color='gray'
              leftSection={<Chevron open={advanced} />}
              onClick={() => setAdvanced(!advanced)}
            >
              {tExt('editor.advanced', {})}
            </Button>
            <Collapse expanded={advanced}>
              <TextInput
                pt='xs'
                label={tExt('editor.condition', {})}
                description={tExt('editor.conditionDescription', {})}
                placeholder='{{ ENABLE_X }}'
                value={step.condition ?? ''}
                onChange={(e) => onChange({ ...step, condition: e.target.value || null })}
                spellCheck={false}
                styles={MONOSPACE}
              />
            </Collapse>
          </div>
        </Stack>
      )}
    </Card>
  );
}
