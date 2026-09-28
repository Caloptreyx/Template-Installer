import Stack from '@/elements/layout/Stack.tsx';
import Drawer from '@/elements/overlays/Drawer.tsx';
import Code from '@/elements/typography/Code.tsx';
import Text from '@/elements/typography/Text.tsx';
import Title from '@/elements/typography/Title.tsx';
import { useExtTranslations } from '../translations.ts';

type HelpEntry = readonly [code: string, key: string];

// template syntax is code, not translatable text; only the explanations are translated
const SECTIONS = [
  {
    key: 'variables',
    entries: [
      ['{{ MOTD }}', 'variable'],
      ['{{ vars.MOTD }}', 'vars'],
    ],
  },
  {
    key: 'server',
    entries: [
      ['{{ server.uuid }}  {{ server.uuid_short }}', 'serverUuid'],
      ['{{ server.name }}  {{ server.description }}', 'serverName'],
      ['{{ server.memory }}  {{ server.swap }}  {{ server.disk }}  {{ server.cpu }}', 'serverLimits'],
      ['{{ server.image }}  {{ server.startup }}', 'serverImage'],
      ['{{ server.ip }}  {{ server.ip_alias }}', 'serverIp'],
      ['{{ server.port }}', 'serverPort'],
      ['{{ server.allocations }}', 'serverAllocations'],
      ['{{ server.egg.name }}  {{ server.nest.name }}  {{ server.node.name }}', 'serverEgg'],
      ['{{ server.env.SERVER_JARFILE }}', 'serverEnv'],
      ['{{ server.timezone }}', 'serverTimezone'],
    ],
  },
  {
    key: 'user',
    entries: [['{{ user.uuid }}  {{ user.username }}  {{ user.email }}', 'user']],
  },
  {
    key: 'panel',
    entries: [['{{ panel.name }}  {{ panel.url }}', 'panel']],
  },
  {
    key: 'functions',
    entries: [
      ['{{ random_string() }}  {{ random_string(32) }}', 'randomString'],
      ['{{ uuid() }}', 'uuid'],
      ['{{ now() }}  {{ now("%Y-%m-%d") }}', 'now'],
    ],
  },
] as const satisfies readonly { key: string; entries: readonly HelpEntry[] }[];

const EXAMPLES = [
  ['motd={{ MOTD }}\nserver-port={{ server.port }}', 'exampleValue'],
  ['{% if ENABLE_X == "1" %}\nenable-x=true\n{% endif %}', 'exampleCondition'],
  ['level-name={{ LEVEL_NAME | default("world", true) }}', 'exampleDefault'],
  [
    '{% for allocation in server.allocations %}\n{{ allocation.ip }}:{{ allocation.port }}\n{% endfor %}',
    'exampleLoop',
  ],
  ['{{ ENABLE_X }}', 'exampleStepCondition'],
] as const;

export default function TemplateHelpDrawer({ opened, onClose }: { opened: boolean; onClose: () => void }) {
  const { t: tExt } = useExtTranslations();

  return (
    <Drawer opened={opened} onClose={onClose} position='right' size='lg' title={tExt('help.title', {})}>
      <Stack gap='lg'>
        <Text size='sm' c='dimmed'>
          {tExt('help.intro', {})}
        </Text>

        {SECTIONS.map((section) => (
          <Stack key={section.key} gap='xs'>
            <Title order={5}>{tExt(`help.sections.${section.key}`, {})}</Title>
            {section.entries.map(([code, key]) => (
              <Stack key={key} gap={2}>
                <Code className='w-fit whitespace-pre-wrap'>{code}</Code>
                <Text size='sm'>{tExt(`help.entries.${key}`, {})}</Text>
              </Stack>
            ))}
          </Stack>
        ))}

        <Stack gap='xs'>
          <Title order={5}>{tExt('help.sections.examples', {})}</Title>
          {EXAMPLES.map(([code, key]) => (
            <Stack key={key} gap={4}>
              <Text size='sm'>{tExt(`help.entries.${key}`, {})}</Text>
              <Code block>{code}</Code>
            </Stack>
          ))}
          <Text size='sm' c='dimmed'>
            {tExt('help.filters', {})}
          </Text>
        </Stack>
      </Stack>
    </Drawer>
  );
}
