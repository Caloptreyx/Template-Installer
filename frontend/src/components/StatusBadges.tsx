import { faCircle, faCircleCheck, faCircleXmark, faForward, faSpinner } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import Badge from '@/elements/data-display/Badge.tsx';
import Tooltip from '@/elements/overlays/Tooltip.tsx';
import type { InstallStatus, StepStatus } from '../api.ts';
import { useExtTranslations } from '../translations.ts';

const INSTALL_COLORS: Record<InstallStatus, string> = {
  running: 'blue',
  success: 'green',
  failed: 'red',
};

export function InstallStatusBadge({ status }: { status: InstallStatus }) {
  const { t: tExt } = useExtTranslations();

  return (
    <Badge
      color={INSTALL_COLORS[status]}
      leftSection={status === 'running' ? <FontAwesomeIcon icon={faSpinner} spin size='xs' /> : undefined}
    >
      {tExt(`installStatus.${status}`, {})}
    </Badge>
  );
}

const STEP_STATUS_ICONS = {
  pending: faCircle,
  running: faSpinner,
  success: faCircleCheck,
  skipped: faForward,
  failed: faCircleXmark,
} as const;

export const STEP_STATUS_COLORS: Record<StepStatus, string> = {
  pending: 'gray',
  running: 'blue',
  success: 'green',
  skipped: 'gray',
  failed: 'red',
};

// icon of a step's status, drawn on a timeline bullet filled with STEP_STATUS_COLORS
export function StepStatusIcon({ status }: { status: StepStatus }) {
  const { t: tExt } = useExtTranslations();

  return (
    <Tooltip label={tExt(`stepStatus.${status}`, {})}>
      <FontAwesomeIcon
        icon={STEP_STATUS_ICONS[status]}
        spin={status === 'running'}
        size={status === 'pending' ? 'xs' : 'sm'}
        style={{ color: status === 'pending' ? 'var(--mantine-color-dimmed)' : 'var(--mantine-color-white)' }}
        fixedWidth
      />
    </Tooltip>
  );
}
