import { httpErrorToHuman } from '@/api/axios.ts';
import Alert from '@/elements/feedback/Alert.tsx';
import MultiSelect from '@/elements/input/MultiSelect.tsx';
import Switch from '@/elements/input/Switch.tsx';
import TextArea from '@/elements/input/TextArea.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import { type Egg, MAX_NAME } from '../../../api.ts';
import { useExtTranslations } from '../../../translations.ts';
import useAdminEggs from '../useAdminEggs.ts';

export interface GeneralValues {
  name: string;
  description: string;
  enabled: boolean;
  eggUuids: string[];
}

function groupEggsByNest(eggs: Egg[]) {
  const groups: { group: string; items: { value: string; label: string }[] }[] = [];
  for (const egg of eggs) {
    let group = groups.find((candidate) => candidate.group === egg.nest.name);
    if (!group) {
      group = { group: egg.nest.name, items: [] };
      groups.push(group);
    }
    group.items.push({ value: egg.uuid, label: egg.name });
  }
  return groups;
}

export default function GeneralTab({
  values,
  onChange,
  showErrors,
}: {
  values: GeneralValues;
  onChange: (patch: Partial<GeneralValues>) => void;
  showErrors: boolean;
}) {
  const { t: tExt } = useExtTranslations();
  const { data: eggs, error: eggsError, isLoading: eggsLoading } = useAdminEggs();

  // uuids of deleted eggs are ignored by the panel; they are dropped once the selection is edited
  const selectedEggs = eggs ? values.eggUuids.filter((uuid) => eggs.some((egg) => egg.uuid === uuid)) : [];

  return (
    <Stack gap='md'>
      <TextInput
        withAsterisk
        label={tExt('editor.name', {})}
        placeholder={tExt('editor.namePlaceholder', {})}
        value={values.name}
        onChange={(e) => onChange({ name: e.target.value })}
        error={showErrors && !values.name.trim() ? tExt('editor.nameRequired', {}) : undefined}
        maxLength={MAX_NAME}
      />
      <TextArea
        label={tExt('editor.description', {})}
        placeholder={tExt('editor.descriptionPlaceholder', {})}
        value={values.description}
        onChange={(e) => onChange({ description: e.target.value })}
        autosize
        minRows={2}
        maxRows={8}
      />
      <Switch
        label={tExt('editor.enabled', {})}
        description={tExt('editor.enabledDescription', {})}
        checked={values.enabled}
        onChange={(e) => onChange({ enabled: e.target.checked })}
      />
      {eggsError && <Alert color='red'>{httpErrorToHuman(eggsError)}</Alert>}
      <MultiSelect
        label={tExt('editor.eggs', {})}
        description={tExt('editor.eggsDescription', {})}
        placeholder={selectedEggs.length ? '' : tExt('editor.eggsPlaceholder', {})}
        data={groupEggsByNest(eggs ?? [])}
        value={selectedEggs}
        onChange={(eggUuids) => onChange({ eggUuids })}
        disabled={eggsLoading || !eggs}
        searchable
        clearable
        hidePickedOptions
      />
    </Stack>
  );
}
