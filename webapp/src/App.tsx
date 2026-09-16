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
const CircuitToolsIndex = lazy(async () => ({ default: (await import('./components/CircuitToolsPage')).CircuitToolsIndex }));
const DividerPage = lazy(async () => {
  const { CircuitToolsPage } = await import('./components/CircuitToolsPage');
  return { default: () => <CircuitToolsPage toolId="divider" /> };
});
const RcPage = lazy(async () => {
  const { CircuitToolsPage } = await import('./components/CircuitToolsPage');
  return { default: () => <CircuitToolsPage toolId="rc_lowpass" /> };
});

type AppView = 'landing' | 'workspace' | 'install' | 'tools' | 'divider' | 'rc';

function viewFromLocation(): AppView {
  const hash = globalThis.location.hash;
  if (hash === '#editor' || hash.startsWith('#kessetsu=')) return 'workspace';
  if (/\/tools\/voltage-divider\/?$/.test(globalThis.location.pathname)) return 'divider';
  if (/\/tools\/rc-lowpass\/?$/.test(globalThis.location.pathname)) return 'rc';
  if (/\/tools\/?$/.test(globalThis.location.pathname)) return 'tools';
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
      {view === 'install' ? <InstallPage /> : view === 'tools' ? <CircuitToolsIndex />
        : view === 'divider' ? <DividerPage /> : view === 'rc' ? <RcPage /> : <WorkspaceApp />}
    </Suspense>
  );
}

export default App;
