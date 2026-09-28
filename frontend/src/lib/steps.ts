import {
  faArrowRightArrowLeft,
  faArrowTurnDown,
  faArrowTurnUp,
  faFileArrowDown,
  faFileCode,
  faFilePen,
  faFileZipper,
  faFolderPlus,
  faFolderTree,
  faPowerOff,
  faTrashCan,
  type IconDefinition,
} from '@fortawesome/free-solid-svg-icons';
import { MAX_CONTENT_BYTES, type Step, type StepOf, type StepType } from '../api.ts';

export const STEP_ICONS: Record<StepType, IconDefinition> = {
  write_file: faFilePen,
  replace_in_file: faFileCode,
  append_file: faArrowTurnDown,
  prepend_file: faArrowTurnUp,
  create_folder: faFolderPlus,
  pull_file: faFileArrowDown,
  unzip_file: faFileZipper,
  move: faArrowRightArrowLeft,
  move_folder_contents: faFolderTree,
  delete: faTrashCan,
  power: faPowerOff,
};

// text: single line, multiline: small monospace textarea, code: file content editor
export type StepFieldKind = 'text' | 'multiline' | 'code' | 'boolean' | 'powerAction';

type StepFieldKey<T extends StepType> = Exclude<keyof StepOf<T>, 'type' | 'name' | 'condition'> & string;
export type AnyStepFieldKey = { [T in StepType]: StepFieldKey<T> }[StepType];

export interface StepFieldDef {
  key: AnyStepFieldKey;
  kind: StepFieldKind;
  required: boolean;
}

type StepField<T extends StepType> = Omit<StepFieldDef, 'key'> & { key: StepFieldKey<T> };

const field = <T extends StepType>(key: StepFieldKey<T>, kind: StepFieldKind, required = false): StepField<T> => ({
  key,
  kind,
  required,
});

export const STEP_FIELDS: { [T in StepType]: StepField<T>[] } = {
  write_file: [field('path', 'text', true), field('content', 'code')],
  replace_in_file: [
    field('path', 'text', true),
    field('search', 'multiline', true),
    field('replace', 'multiline'),
    field('regex', 'boolean'),
  ],
  append_file: [field('path', 'text', true), field('content', 'code')],
  prepend_file: [field('path', 'text', true), field('content', 'code')],
  create_folder: [field('path', 'text', true)],
  pull_file: [field('url', 'text', true), field('directory', 'text', true), field('fileName', 'text')],
  unzip_file: [field('path', 'text', true), field('destination', 'text'), field('deleteArchive', 'boolean')],
  move: [field('from', 'text', true), field('to', 'text', true)],
  move_folder_contents: [field('from', 'text', true), field('to', 'text', true)],
  delete: [field('path', 'text', true)],
  power: [field('action', 'powerAction', true), field('wait', 'boolean')],
};

const STEP_DEFAULTS: { [T in StepType]: Omit<StepOf<T>, 'type' | 'name' | 'condition'> } = {
  write_file: { path: '', content: '' },
  replace_in_file: { path: '', search: '', replace: '', regex: false },
  append_file: { path: '', content: '' },
  prepend_file: { path: '', content: '' },
  create_folder: { path: '' },
  pull_file: { url: '', directory: '/', fileName: '' },
  unzip_file: { path: '', destination: '', deleteArchive: false },
  move: { from: '', to: '' },
  move_folder_contents: { from: '', to: '' },
  delete: { path: '' },
  power: { action: 'restart', wait: true },
};

export function createStep(type: StepType): Step {
  return { type, name: null, condition: null, ...STEP_DEFAULTS[type] } as Step;
}

export function getStepValue(step: Step, key: string): unknown {
  return (step as Record<string, unknown>)[key];
}

export function setStepValue(step: Step, key: string, value: unknown): Step {
  return { ...step, [key]: value } as Step;
}

export type StepFieldError = 'fieldRequired' | 'contentTooLarge';

export function stepFieldErrors(step: Step): Record<string, StepFieldError> {
  const fields: readonly StepFieldDef[] = STEP_FIELDS[step.type];
  const errors: Record<string, StepFieldError> = {};
  for (const field of fields) {
    const value = getStepValue(step, field.key);
    if (typeof value !== 'string') continue;

    if (field.required && !value.trim()) {
      errors[field.key] = 'fieldRequired';
    } else if (
      field.kind === 'code' &&
      // a UTF-16 code unit takes at most 3 UTF-8 bytes; only encode contents that could exceed the limit
      value.length * 3 > MAX_CONTENT_BYTES &&
      new TextEncoder().encode(value).length > MAX_CONTENT_BYTES
    ) {
      errors[field.key] = 'contentTooLarge';
    }
  }
  return errors;
}

// short, untranslated hint of what a step touches, shown on collapsed step cards
export function stepSummary(step: Step): string {
  switch (step.type) {
    case 'pull_file':
      return step.url;
    case 'move':
    case 'move_folder_contents':
      return step.from || step.to ? `${step.from} → ${step.to}` : '';
    case 'power':
      return '';
    default:
      return step.path;
  }
}

const EDITOR_LANGUAGES: Record<string, string> = {
  json: 'json',
  yml: 'yaml',
  yaml: 'yaml',
  toml: 'ini',
  ini: 'ini',
  cfg: 'ini',
  conf: 'ini',
  properties: 'ini',
  env: 'ini',
  xml: 'xml',
  html: 'html',
  htm: 'html',
  css: 'css',
  js: 'javascript',
  mjs: 'javascript',
  ts: 'typescript',
  lua: 'lua',
  py: 'python',
  sh: 'shell',
  bash: 'shell',
  md: 'markdown',
  sql: 'sql',
  php: 'php',
};

// the editor language for a (templated) file path, based on its extension
export function editorLanguage(path: string): string {
  const extension = path.split('/').pop()?.split('.').pop()?.toLowerCase() ?? '';
  return EDITOR_LANGUAGES[extension] ?? 'plaintext';
}
