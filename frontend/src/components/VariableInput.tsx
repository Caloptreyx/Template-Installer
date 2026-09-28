import NumberInput from '@/elements/input/NumberInput.tsx';
import Select from '@/elements/input/Select.tsx';
import Switch from '@/elements/input/Switch.tsx';
import TextArea from '@/elements/input/TextArea.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import type { Variable } from '../api.ts';
import { inferVariableInput, isRequired } from '../lib/variables.ts';

// renders the input matching a variable's rules; every value is a string ("1"/"0" for booleans)
export default function VariableInput({
  variable,
  value,
  onChange,
  error,
}: {
  variable: Variable;
  value: string;
  onChange: (value: string) => void;
  error?: string;
}) {
  const input = inferVariableInput(variable);
  const common = {
    label: variable.label,
    description: variable.description ?? undefined,
    withAsterisk: isRequired(variable),
    error,
  };

  switch (input.kind) {
    case 'select':
      return (
        <Select
          {...common}
          data={input.options}
          value={value || null}
          onChange={(option) => onChange(option ?? '')}
          allowDeselect={!common.withAsterisk}
          clearable={!common.withAsterisk}
          searchable={input.options.length > 8}
        />
      );
    case 'boolean':
      return (
        <Switch
          label={common.withAsterisk ? `${variable.label} *` : variable.label}
          description={common.description}
          checked={value === '1'}
          onChange={(e) => onChange(e.target.checked ? '1' : '0')}
          error={error}
        />
      );
    case 'number':
      return <NumberInput {...common} value={value} onChange={(number) => onChange(String(number))} />;
    case 'textarea':
      return (
        <TextArea
          {...common}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          autosize
          minRows={3}
          maxRows={12}
        />
      );
    default:
      return <TextInput {...common} value={value} onChange={(e) => onChange(e.target.value)} />;
  }
}
