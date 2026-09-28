import { z } from 'zod';
import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi, parsePaginationFromApi, serializeForApi } from '@/lib/serialization/api-transform.ts';

export const TEMPLATES_ADMIN_BASE = '/api/admin/extensions/dev.caloptreyx.templates';
export const templatesServerBase = (serverUuid: string) => `/api/client/servers/${serverUuid}/templates`;

export const MAX_STEPS = 100;
export const MAX_VARIABLES = 50;
export const MAX_NAME = 255;
export const MAX_LABEL = 128;
export const MAX_VARIABLE_DESCRIPTION = 1024;
export const MAX_CONTENT_BYTES = 1024 * 1024;
export const VARIABLE_KEY_REGEX = /^[A-Za-z_][A-Za-z0-9_]{0,63}$/;
// context names and template functions a variable key may not shadow
export const RESERVED_VARIABLE_KEYS = [
  'server',
  'user',
  'panel',
  'vars',
  'loop',
  'self',
  'random_string',
  'uuid',
  'now',
];

export const stepTypeSchema = z.enum([
  'write_file',
  'replace_in_file',
  'append_file',
  'prepend_file',
  'create_folder',
  'pull_file',
  'unzip_file',
  'move',
  'move_folder_contents',
  'delete',
  'power',
]);
export type StepType = z.infer<typeof stepTypeSchema>;
export const STEP_TYPES = stepTypeSchema.options;

export const powerActionSchema = z.enum(['start', 'stop', 'restart', 'kill']);
export type PowerAction = z.infer<typeof powerActionSchema>;
export const POWER_ACTIONS = powerActionSchema.options;

// optional step texts are always sent as strings; an empty string means "not set"
const optionalText = z
  .string()
  .nullish()
  .transform((value) => value ?? '');

const stepCommon = {
  name: z.string().nullable().default(null),
  condition: z.string().nullable().default(null),
};

// keys are camelCased by parseFromApi (file_name -> fileName) and snake_cased back by serializeForApi;
// the `type` values stay untouched
export const stepSchema = z.discriminatedUnion('type', [
  z.object({ type: z.literal('write_file'), ...stepCommon, path: z.string(), content: z.string() }),
  z.object({
    type: z.literal('replace_in_file'),
    ...stepCommon,
    path: z.string(),
    search: z.string(),
    replace: z.string(),
    regex: z.boolean().default(false),
  }),
  z.object({ type: z.literal('append_file'), ...stepCommon, path: z.string(), content: z.string() }),
  z.object({ type: z.literal('prepend_file'), ...stepCommon, path: z.string(), content: z.string() }),
  z.object({ type: z.literal('create_folder'), ...stepCommon, path: z.string() }),
  z.object({
    type: z.literal('pull_file'),
    ...stepCommon,
    url: z.string(),
    directory: z.string(),
    fileName: optionalText,
  }),
  z.object({
    type: z.literal('unzip_file'),
    ...stepCommon,
    path: z.string(),
    destination: optionalText,
    deleteArchive: z.boolean().default(false),
  }),
  z.object({ type: z.literal('move'), ...stepCommon, from: z.string(), to: z.string() }),
  z.object({ type: z.literal('move_folder_contents'), ...stepCommon, from: z.string(), to: z.string() }),
  z.object({ type: z.literal('delete'), ...stepCommon, path: z.string() }),
  z.object({ type: z.literal('power'), ...stepCommon, action: powerActionSchema, wait: z.boolean().default(true) }),
]);
export type Step = z.infer<typeof stepSchema>;
export type StepOf<T extends StepType> = Extract<Step, { type: T }>;

export const variableSchema = z.object({
  key: z.string(),
  label: z.string(),
  description: z.string().nullable().default(null),
  defaultValue: z.string().default(''),
  rules: z.array(z.string()).default([]),
});
export type Variable = z.infer<typeof variableSchema>;

export const templateSchema = z.object({
  uuid: z.string(),
  name: z.string(),
  description: z.string().nullable(),
  enabled: z.boolean(),
  eggUuids: z.array(z.string()),
  variables: z.array(variableSchema),
  steps: z.array(stepSchema),
  created: z.coerce.date(),
  updated: z.coerce.date(),
});
export type Template = z.infer<typeof templateSchema>;

export const templatePayloadSchema = z.object({
  name: z.string().min(1).max(MAX_NAME),
  description: z.string().nullable(),
  enabled: z.boolean(),
  eggUuids: z.array(z.string()),
  variables: z.array(variableSchema).max(MAX_VARIABLES),
  steps: z.array(stepSchema).max(MAX_STEPS),
});
export type TemplatePayload = z.infer<typeof templatePayloadSchema>;

export const eggSchema = z.object({
  uuid: z.string(),
  name: z.string(),
  nest: z.object({ uuid: z.string(), name: z.string() }),
});
export type Egg = z.infer<typeof eggSchema>;

export const clientTemplateSchema = z.object({
  uuid: z.string(),
  name: z.string(),
  description: z.string().nullable(),
  variables: z.array(variableSchema),
  steps: z.array(z.object({ type: stepTypeSchema, name: z.string().nullable() })),
});
export type ClientTemplate = z.infer<typeof clientTemplateSchema>;

export const stepStatusSchema = z.enum(['pending', 'running', 'success', 'skipped', 'failed']);
export type StepStatus = z.infer<typeof stepStatusSchema>;

export const installStatusSchema = z.enum(['running', 'success', 'failed']);
export type InstallStatus = z.infer<typeof installStatusSchema>;

export const stepProgressSchema = z.object({
  index: z.number().int(),
  type: stepTypeSchema,
  name: z.string().nullable(),
  status: stepStatusSchema,
  message: z.string().nullable(),
});
export type StepProgress = z.infer<typeof stepProgressSchema>;

export const installSchema = z.object({
  uuid: z.string(),
  templateUuid: z.string().nullable(),
  templateName: z.string(),
  user: z.object({ uuid: z.string(), username: z.string() }).nullable(),
  status: installStatusSchema,
  steps: z.array(stepProgressSchema),
  error: z.string().nullable(),
  created: z.coerce.date(),
  finished: z.coerce.date().nullable(),
});
export type Install = z.infer<typeof installSchema>;

// variables travel as an array so their (user-defined) keys never pass through the key conversion
export const installRequestSchema = z.object({
  variables: z.array(z.object({ key: z.string(), value: z.string() })),
});
export type InstallRequest = z.infer<typeof installRequestSchema>;

export const templatesQueryKey = ['dev.caloptreyx.templates'] as const;

// admin

export const getAdminTemplates = async (page: number, search: string): Promise<Pagination<Template>> => {
  const { data } = await axiosInstance.get(`${TEMPLATES_ADMIN_BASE}/templates`, {
    params: { page, search: search || undefined },
  });
  return parsePaginationFromApi(templateSchema, data.templates);
};

export const createAdminTemplate = async (payload: TemplatePayload): Promise<Template> => {
  const { data } = await axiosInstance.post(
    `${TEMPLATES_ADMIN_BASE}/templates`,
    serializeForApi(templatePayloadSchema, payload),
  );
  return parseFromApi(templateSchema, data.template);
};

export const updateAdminTemplate = async (templateUuid: string, payload: TemplatePayload): Promise<Template> => {
  const { data } = await axiosInstance.patch(
    `${TEMPLATES_ADMIN_BASE}/templates/${templateUuid}`,
    serializeForApi(templatePayloadSchema, payload),
  );
  return parseFromApi(templateSchema, data.template);
};

export const deleteAdminTemplate = async (templateUuid: string): Promise<void> => {
  await axiosInstance.delete(`${TEMPLATES_ADMIN_BASE}/templates/${templateUuid}`);
};

// the export document is returned as the exact text the backend produced, without any key conversion
export const exportAdminTemplate = async (templateUuid: string): Promise<string> => {
  const { data } = await axiosInstance.get<string>(`${TEMPLATES_ADMIN_BASE}/templates/${templateUuid}/export`, {
    responseType: 'text',
    transformResponse: [(raw: string) => raw],
  });
  return data;
};

// the uploaded export document is sent verbatim
export const importAdminTemplate = async (exportDocument: string): Promise<Template> => {
  const { data } = await axiosInstance.post(`${TEMPLATES_ADMIN_BASE}/templates/import`, exportDocument, {
    headers: { 'Content-Type': 'application/json' },
  });
  return parseFromApi(templateSchema, data.template);
};

export const getAdminEggs = async (): Promise<Egg[]> => {
  const { data } = await axiosInstance.get(`${TEMPLATES_ADMIN_BASE}/eggs`);
  return parseFromApi(z.array(eggSchema), data.eggs);
};

// server

export const getServerTemplates = async (serverUuid: string): Promise<ClientTemplate[]> => {
  const { data } = await axiosInstance.get(templatesServerBase(serverUuid));
  return parseFromApi(z.array(clientTemplateSchema), data.templates);
};

export const installServerTemplate = async (
  serverUuid: string,
  templateUuid: string,
  request: InstallRequest,
): Promise<Install> => {
  const { data } = await axiosInstance.post(
    `${templatesServerBase(serverUuid)}/${templateUuid}/install`,
    serializeForApi(installRequestSchema, request),
  );
  return parseFromApi(installSchema, data.install);
};

export const getServerInstalls = async (serverUuid: string): Promise<Install[]> => {
  const { data } = await axiosInstance.get(`${templatesServerBase(serverUuid)}/installs`);
  return parseFromApi(z.array(installSchema), data.installs);
};

export const getServerInstall = async (serverUuid: string, installUuid: string): Promise<Install> => {
  const { data } = await axiosInstance.get(`${templatesServerBase(serverUuid)}/installs/${installUuid}`);
  return parseFromApi(installSchema, data.install);
};
