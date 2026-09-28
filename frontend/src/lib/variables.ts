import { RESERVED_VARIABLE_KEYS, VARIABLE_KEY_REGEX, type Variable } from '../api.ts';

export type VariableInput =
  | { kind: 'select'; options: string[] }
  | { kind: 'boolean' }
  | { kind: 'number' }
  | { kind: 'textarea' }
  | { kind: 'text' };

// rules shown as a cheat sheet next to the rules input
export const COMMON_RULES = [
  'required',
  'nullable',
  'string',
  'integer',
  'numeric',
  'boolean',
  'min:1',
  'max:32',
  'between:1,100',
  'in:a,b,c',
  'regex:/^[a-z0-9_]+$/',
];

const TRUE_VALUES = ['1', 'true', 'on', 'yes'];

const ruleName = (rule: string) => rule.split(':')[0].trim().toLowerCase();

export const hasRule = (variable: Variable, name: string) => variable.rules.some((rule) => ruleName(rule) === name);

export const isRequired = (variable: Variable) => hasRule(variable, 'required');

function inOptions(variable: Variable): string[] | null {
  const rule = variable.rules.find((rule) => ruleName(rule) === 'in');
  if (!rule) return null;

  return rule
    .slice(rule.indexOf(':') + 1)
    .split(',')
    .map((option) => option.trim().replace(/^"(.*)"$/, '$1'))
    .filter((option) => option.length > 0);
}

export function inferVariableInput(variable: Variable): VariableInput {
  const options = inOptions(variable);
  if (options) return { kind: 'select', options };
  if (hasRule(variable, 'boolean')) return { kind: 'boolean' };
  if (hasRule(variable, 'integer') || hasRule(variable, 'numeric')) return { kind: 'number' };
  return variable.defaultValue.includes('\n') ? { kind: 'textarea' } : { kind: 'text' };
}

// the value a variable input starts with; boolean inputs always submit "1" or "0"
export function initialVariableValue(variable: Variable): string {
  if (inferVariableInput(variable).kind === 'boolean') {
    return TRUE_VALUES.includes(variable.defaultValue.trim().toLowerCase()) ? '1' : '0';
  }
  return variable.defaultValue;
}

export type VariableKeyError = 'keyRequired' | 'keyInvalid' | 'keyReserved' | 'keyDuplicate';

export function variableKeyError(variables: Variable[], index: number): VariableKeyError | null {
  const key = variables[index].key;
  if (!key) return 'keyRequired';
  if (!VARIABLE_KEY_REGEX.test(key)) return 'keyInvalid';
  if (RESERVED_VARIABLE_KEYS.includes(key)) return 'keyReserved';
  if (variables.some((variable, i) => i < index && variable.key === key)) return 'keyDuplicate';
  return null;
}
