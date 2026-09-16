import { lazy, Suspense, useEffect, useState } from 'react';
import { LandingPage } from './components/LandingPage';

const WorkspaceApp = lazy(async () => {
  const module = await import('./components/WorkspaceApp');
  return { default: module.WorkspaceApp };
});
const InstallPage = lazy(async () => {
  const module = await import('./components/InstallPage');
  return { default: module.InstallPage };
});

type AppView = 'landing' | 'workspace' | 'install';

function viewFromLocation(): AppView {
  const hash = globalThis.location.hash;
  if (hash === '#editor' || hash.startsWith('#kessetsu=')) return 'workspace';
  return hash === '#install' || /\/install\/?$/.test(globalThis.location.pathname) ? 'install' : 'landing';
}

function App() {
  const [view, setView] = useState<AppView>(viewFromLocation);

  useEffect(() => {
    const updateView = () => setView(viewFromLocation());
    globalThis.addEventListener('hashchange', updateView);
    return () => globalThis.removeEventListener('hashchange', updateView);
  }, []);

  if (view === 'landing') return <LandingPage />;

  return (
    <Suspense fallback={<div className="workspace-loading" role="status">Loading Kessetsu Core…</div>}>
      {view === 'install' ? <InstallPage /> : <WorkspaceApp />}
    </Suspense>
  );
}

export default App;
