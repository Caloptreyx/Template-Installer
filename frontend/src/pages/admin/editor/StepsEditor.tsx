import { faPlus } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { useState } from 'react';
import Button from '@/elements/buttons/Button.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import Group from '@/elements/layout/Group.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import Menu from '@/elements/overlays/Menu.tsx';
import Text from '@/elements/typography/Text.tsx';
import { MAX_STEPS, STEP_TYPES, type Step } from '../../../api.ts';
import { createStep, STEP_ICONS, stepFieldErrors } from '../../../lib/steps.ts';
import { useExtTranslations } from '../../../translations.ts';
import { type Keyed, keyed, moveItem } from './keyedList.ts';
import StepCard from './StepCard.tsx';

export default function StepsEditor({
  value,
  onChange,
  showErrors,
}: {
  value: Keyed<Step>[];
  onChange: (steps: Keyed<Step>[]) => void;
  showErrors: boolean;
}) {
  const { t: tExt } = useExtTranslations();

  // steps added in this session start expanded, loaded ones collapsed
  const [addedIds, setAddedIds] = useState<string[]>([]);

  const canAdd = value.length < MAX_STEPS;

  const insert = (index: number, step: Step) => {
    const row = keyed(step);
    setAddedIds((ids) => [...ids, row.id]);
    onChange([...value.slice(0, index), row, ...value.slice(index)]);
  };

  return (
    <Stack gap='sm'>
      <Alert color='blue'>{tExt('editor.stepsHint', {})}</Alert>

      {value.length === 0 && (
        <Text size='sm' c='dimmed'>
          {tExt('editor.stepsEmpty', {})}
        </Text>
      )}

      {value.map((row, index) => (
        <StepCard
          key={row.id}
          step={row.value}
          index={index}
          count={value.length}
          errors={stepFieldErrors(row.value)}
          showErrors={showErrors}
          initiallyExpanded={addedIds.includes(row.id)}
          canDuplicate={canAdd}
          onChange={(step) =>
            onChange(value.map((current, i) => (i === index ? { ...current, value: step } : current)))
          }
          onMove={(offset) => onChange(moveItem(value, index, offset))}
          onDuplicate={() => insert(index + 1, structuredClone(row.value))}
          onRemove={() => onChange(value.filter((_, i) => i !== index))}
        />
      ))}

      <Group>
        <Menu shadow='md' width={320} position='bottom-start'>
          <Menu.Target>
            <Button variant='default' leftSection={<FontAwesomeIcon icon={faPlus} />} disabled={!canAdd}>
              {tExt('editor.addStep', {})}
            </Button>
          </Menu.Target>
          <Menu.Dropdown mah='70vh' style={{ overflowY: 'auto' }}>
            {STEP_TYPES.map((type) => (
              <Menu.Item
                key={type}
                leftSection={<FontAwesomeIcon icon={STEP_ICONS[type]} fixedWidth />}
                onClick={() => insert(value.length, createStep(type))}
              >
                <Text size='sm'>{tExt(`steps.types.${type}.label`, {})}</Text>
                <Text size='xs' c='dimmed'>
                  {tExt(`steps.types.${type}.description`, {})}
                </Text>
              </Menu.Item>
            ))}
          </Menu.Dropdown>
        </Menu>
        {!canAdd && (
          <Text size='sm' c='dimmed'>
            {tExt('editor.stepsLimit', { max: MAX_STEPS })}
          </Text>
        )}
      </Group>
    </Stack>
  );
}
