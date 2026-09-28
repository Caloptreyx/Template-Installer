import { faArrowDown, faArrowUp, faCopy, faTrash, type IconDefinition } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import ActionIcon from '@/elements/buttons/ActionIcon.tsx';
import Group from '@/elements/layout/Group.tsx';
import Tooltip from '@/elements/overlays/Tooltip.tsx';
import { useExtTranslations } from '../../../translations.ts';

function Action({
  icon,
  label,
  color = 'gray',
  disabled,
  onClick,
}: {
  icon: IconDefinition;
  label: string;
  color?: string;
  disabled?: boolean;
  onClick: () => void;
}) {
  return (
    <Tooltip label={label}>
      <ActionIcon
        variant='subtle'
        color={color}
        disabled={disabled}
        aria-label={label}
        onClick={(e) => {
          e.stopPropagation();
          onClick();
        }}
      >
        <FontAwesomeIcon icon={icon} />
      </ActionIcon>
    </Tooltip>
  );
}

// move up/down, duplicate and remove buttons of a variable row or step card
export default function ItemActions({
  index,
  count,
  onMove,
  onDuplicate,
  duplicateDisabled,
  onRemove,
}: {
  index: number;
  count: number;
  onMove: (offset: -1 | 1) => void;
  onDuplicate?: () => void;
  duplicateDisabled?: boolean;
  onRemove: () => void;
}) {
  const { t: tExt } = useExtTranslations();

  return (
    <Group gap={2} wrap='nowrap'>
      <Action icon={faArrowUp} label={tExt('editor.moveUp', {})} disabled={index === 0} onClick={() => onMove(-1)} />
      <Action
        icon={faArrowDown}
        label={tExt('editor.moveDown', {})}
        disabled={index === count - 1}
        onClick={() => onMove(1)}
      />
      {onDuplicate && (
        <Action icon={faCopy} label={tExt('editor.duplicate', {})} disabled={duplicateDisabled} onClick={onDuplicate} />
      )}
      <Action icon={faTrash} label={tExt('editor.remove', {})} color='red' onClick={onRemove} />
    </Group>
  );
}
