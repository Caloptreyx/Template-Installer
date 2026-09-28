import { faPlus } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { SimpleGrid, UnstyledButton } from '@mantine/core';
import Button from '@/elements/buttons/Button.tsx';
import Card from '@/elements/data-display/Card.tsx';
import TagsInput from '@/elements/input/TagsInput.tsx';
import TextArea from '@/elements/input/TextArea.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import Group from '@/elements/layout/Group.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import Code from '@/elements/typography/Code.tsx';
import Text from '@/elements/typography/Text.tsx';
import { MAX_LABEL, MAX_VARIABLE_DESCRIPTION, MAX_VARIABLES, type Variable } from '../../../api.ts';
import { COMMON_RULES, variableKeyError } from '../../../lib/variables.ts';
import { useExtTranslations } from '../../../translations.ts';
import ItemActions from './ItemActions.tsx';
import { type Keyed, keyed, moveItem } from './keyedList.ts';

export default function VariablesEditor({
  value,
  onChange,
  showErrors,
}: {
  value: Keyed<Variable>[];
  onChange: (variables: Keyed<Variable>[]) => void;
  showErrors: boolean;
}) {
  const { t: tExt } = useExtTranslations();

  const variables = value.map((row) => row.value);
  const update = (index: number, patch: Partial<Variable>) =>
    onChange(value.map((row, i) => (i === index ? { ...row, value: { ...row.value, ...patch } } : row)));

  return (
    <Stack gap='md'>
      {value.length === 0 && (
        <Text size='sm' c='dimmed'>
          {tExt('variables.empty', {})}
        </Text>
      )}

      {value.map((row, index) => {
        const variable = row.value;
        const keyError = variableKeyError(variables, index);

        return (
          <Card key={row.id} p='md'>
            <Stack gap='sm'>
              <Group justify='space-between' wrap='nowrap'>
                <Code>{variable.key || `#${index + 1}`}</Code>
                <ItemActions
                  index={index}
                  count={value.length}
                  onMove={(offset) => onChange(moveItem(value, index, offset))}
                  onRemove={() => onChange(value.filter((_, i) => i !== index))}
                />
              </Group>

              <SimpleGrid cols={{ base: 1, md: 3 }}>
                <TextInput
                  withAsterisk
                  label={tExt('variables.key', {})}
                  placeholder={tExt('variables.keyPlaceholder', {})}
                  value={variable.key}
                  onChange={(e) => update(index, { key: e.target.value.trim() })}
                  error={
                    keyError && (showErrors || keyError !== 'keyRequired')
                      ? tExt(`variables.${keyError}`, {})
                      : undefined
                  }
                  ff='monospace'
                  maxLength={64}
                />
                <TextInput
                  withAsterisk
                  label={tExt('variables.label', {})}
                  value={variable.label}
                  onChange={(e) => update(index, { label: e.target.value })}
                  error={showErrors && !variable.label.trim() ? tExt('variables.labelRequired', {}) : undefined}
                  maxLength={MAX_LABEL}
                />
                <TextArea
                  label={tExt('variables.defaultValue', {})}
                  value={variable.defaultValue}
                  onChange={(e) => update(index, { defaultValue: e.target.value })}
                  autosize
                  minRows={1}
                  maxRows={8}
                />
              </SimpleGrid>

              <TextArea
                label={tExt('variables.description', {})}
                value={variable.description ?? ''}
                onChange={(e) => update(index, { description: e.target.value || null })}
                autosize
                minRows={1}
                maxRows={6}
                maxLength={MAX_VARIABLE_DESCRIPTION}
              />

              <TagsInput
                label={tExt('variables.rules', {})}
                description={tExt('variables.rulesDescription', {})}
                placeholder={tExt('variables.rulesPlaceholder', {})}
                value={variable.rules}
                onChange={(rules) => update(index, { rules })}
              />
              <Group gap={6}>
                <Text size='xs' c='dimmed'>
                  {tExt('variables.rulesHint', {})}
                </Text>
                {COMMON_RULES.map((rule) => (
                  <UnstyledButton
                    key={rule}
                    onClick={() =>
                      !variable.rules.includes(rule) && update(index, { rules: [...variable.rules, rule] })
                    }
                  >
                    <Code>{rule}</Code>
                  </UnstyledButton>
                ))}
              </Group>
            </Stack>
          </Card>
        );
      })}

      <Group>
        <Button
          variant='default'
          leftSection={<FontAwesomeIcon icon={faPlus} />}
          onClick={() =>
            onChange([
              ...value,
              keyed<Variable>({ key: '', label: '', description: null, defaultValue: '', rules: [] }),
            ])
          }
          disabled={value.length >= MAX_VARIABLES}
        >
          {tExt('variables.add', {})}
        </Button>
        {value.length >= MAX_VARIABLES && (
          <Text size='sm' c='dimmed'>
            {tExt('variables.limit', { max: MAX_VARIABLES })}
          </Text>
        )}
      </Group>
    </Stack>
  );
}
