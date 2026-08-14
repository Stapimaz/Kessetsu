import { lazy, Suspense, useEffect, useState } from 'react';
import { LandingPage } from './components/LandingPage';

const WorkspaceApp = lazy(async () => {
  const module = await import('./components/WorkspaceApp');
  return { default: module.WorkspaceApp };
});

type AppView = 'landing' | 'workspace';

function viewFromLocation(): AppView {
  const hash = globalThis.location.hash;
  return hash === '#editor' || hash.startsWith('#kessetsu=') ? 'workspace' : 'landing';
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
      <WorkspaceApp />
    </Suspense>
  );
}

export default App;
