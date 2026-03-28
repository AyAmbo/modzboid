import { Route, Switch, Redirect } from "wouter";
import { Suspense, lazy } from "react";

const ModManagerPage = lazy(
  () => import("../features/mod-manager/pages/ModManagerPage")
);
const ProfilesPage = lazy(
  () => import("../features/profiles/pages/ProfilesPage")
);
const SettingsPage = lazy(
  () => import("../features/settings/pages/SettingsPage")
);
const FirstRunWizard = lazy(
  () => import("../features/first-run/pages/FirstRunWizard")
);
const ServerPage = lazy(
  () => import("../features/server/pages/ServerPage")
);
const BackupPage = lazy(
  () => import("../features/backup/pages/BackupPage")
);
const DependencyGraphPage = lazy(
  () => import("../features/mod-manager/pages/DependencyGraphPage")
);
const ExtensionsPage = lazy(
  () => import("../features/extensions/pages/ExtensionsPage")
);
const DiagnosticsPage = lazy(
  () => import("../features/diagnostics/pages/DiagnosticsPage")
);
const CompatibilityPage = lazy(
  () => import("../features/compatibility/pages/CompatibilityPage")
);
const ApiDocsPage = lazy(
  () => import("../features/api-docs/pages/ApiDocsPage")
);

export function AppRouter() {
  return (
    <Suspense fallback={<div className="p-4">Loading...</div>}>
      <Switch>
        <Route path="/mods" component={ModManagerPage} />
        <Route path="/profiles" component={ProfilesPage} />
        <Route path="/settings" component={SettingsPage} />
        <Route path="/server" component={ServerPage} />
        <Route path="/backups" component={BackupPage} />
        <Route path="/graph" component={DependencyGraphPage} />
        <Route path="/extensions" component={ExtensionsPage} />
        <Route path="/diagnostics" component={DiagnosticsPage} />
        <Route path="/compatibility" component={CompatibilityPage} />
        <Route path="/api-docs" component={ApiDocsPage} />
        <Route path="/first-run" component={FirstRunWizard} />
        <Route>
          <Redirect to="/mods" />
        </Route>
      </Switch>
    </Suspense>
  );
}
