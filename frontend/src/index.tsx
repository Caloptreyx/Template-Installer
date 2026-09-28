import { faWandMagicSparkles } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Extension, ExtensionContext } from 'shared';
import AdminConfigurationPage from './pages/admin/AdminConfigurationPage.tsx';
import ServerTemplatesPage from './pages/server/ServerTemplatesPage.tsx';
import { getExtTranslations } from './translations.ts';

class CaloptreyxTemplatesExtension extends Extension {
  public cardConfigurationPage: React.FC | null = AdminConfigurationPage;
  public cardIcon: React.ReactNode = <FontAwesomeIcon icon={faWandMagicSparkles} />;

  public initialize(ctx: ExtensionContext): void {
    ctx.extensionRegistry.enterRoutes((routes) =>
      routes.addServerRoute({
        name: () => getExtTranslations().t('common.templates', {}),
        icon: faWandMagicSparkles,
        path: '/templates',
        element: ServerTemplatesPage,
        permission: 'templates.read',
      }),
    );

    ctx.extensionRegistry.enterPermissionIcons((icons) =>
      icons
        .addServerPermissionIcon('templates', <FontAwesomeIcon icon={faWandMagicSparkles} />)
        .addAdminPermissionIcon('templates', <FontAwesomeIcon icon={faWandMagicSparkles} />),
    );
  }
}

export default new CaloptreyxTemplatesExtension();
