import { faWandMagicSparkles } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import Button from '@/elements/buttons/Button.tsx';
import { ServerCan } from '@/elements/Can.tsx';
import Badge from '@/elements/data-display/Badge.tsx';
import Card from '@/elements/data-display/Card.tsx';
import ThemeIcon from '@/elements/data-display/ThemeIcon.tsx';
import Group from '@/elements/layout/Group.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import Tooltip from '@/elements/overlays/Tooltip.tsx';
import Text from '@/elements/typography/Text.tsx';
import Title from '@/elements/typography/Title.tsx';
import type { ClientTemplate } from '../../api.ts';
import { STEP_ICONS } from '../../lib/steps.ts';
import { useExtTranslations } from '../../translations.ts';

export default function TemplateCard({ template, onInstall }: { template: ClientTemplate; onInstall: () => void }) {
  const { t: tExt, tItem } = useExtTranslations();

  const stepTypes = [...new Set(template.steps.map((step) => step.type))];

  return (
    <Card p='md' className='h-full'>
      <Stack gap='sm' justify='space-between' h='100%'>
        <Stack gap='sm'>
          <Group gap='sm' wrap='nowrap'>
            <ThemeIcon size='lg' variant='light'>
              <FontAwesomeIcon icon={faWandMagicSparkles} />
            </ThemeIcon>
            <Title order={4} className='min-w-0 truncate'>
              {template.name}
            </Title>
          </Group>
          {template.description && (
            <Text size='sm' c='dimmed' lineClamp={4} className='whitespace-pre-wrap'>
              {template.description}
            </Text>
          )}
        </Stack>

        <Stack gap='sm'>
          <Group gap='xs'>
            <Badge variant='light' color='gray'>
              {tItem('variable', template.variables.length)}
            </Badge>
            <Badge variant='light' color='gray'>
              {tItem('step', template.steps.length)}
            </Badge>
            {stepTypes.map((type) => (
              <Tooltip key={type} label={tExt(`steps.types.${type}.label`, {})}>
                <FontAwesomeIcon icon={STEP_ICONS[type]} size='sm' style={{ color: 'var(--mantine-color-dimmed)' }} />
              </Tooltip>
            ))}
          </Group>
          <ServerCan action='templates.install'>
            <Button fullWidth leftSection={<FontAwesomeIcon icon={faWandMagicSparkles} />} onClick={onInstall}>
              {tExt('install.install', {})}
            </Button>
          </ServerCan>
        </Stack>
      </Stack>
    </Card>
  );
}
