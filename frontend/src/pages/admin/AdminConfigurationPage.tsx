import { useState } from 'react';
import type { Template } from '../../api.ts';
import TemplateHelpDrawer from '../../components/TemplateHelpDrawer.tsx';
import TemplateEditor from './editor/TemplateEditor.tsx';
import TemplatesTable from './TemplatesTable.tsx';

// the editor replaces the table while a template is created or edited
type Editing = { kind: 'create' } | { kind: 'edit'; template: Template } | null;

export default function AdminConfigurationPage() {
  const [editing, setEditing] = useState<Editing>(null);
  const [helpOpen, setHelpOpen] = useState(false);

  return (
    <>
      <TemplateHelpDrawer opened={helpOpen} onClose={() => setHelpOpen(false)} />

      {editing ? (
        <TemplateEditor
          template={editing.kind === 'edit' ? editing.template : undefined}
          onClose={() => setEditing(null)}
          onHelp={() => setHelpOpen(true)}
        />
      ) : (
        <TemplatesTable
          onCreate={() => setEditing({ kind: 'create' })}
          onEdit={(template) => setEditing({ kind: 'edit', template })}
          onHelp={() => setHelpOpen(true)}
        />
      )}
    </>
  );
}
