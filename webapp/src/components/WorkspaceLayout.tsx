import { Activity, CircuitBoard, Code2 } from 'lucide-react';
import { useEffect, useRef, useState, type ReactNode } from 'react';
import type { PanelWindowControls } from './PanelHeader';

type Panel = 'source' | 'schematic' | 'results';
type PanelRenderer = (controls: PanelWindowControls) => ReactNode;
interface LayoutState {
  horizontal: number;
  vertical: number;
  source: boolean;
  schematic: boolean;
  results: boolean;
  maximized: Panel | null;
}

const defaults: LayoutState = {
  horizontal: 40,
  vertical: 55,
  source: true,
  schematic: true,
  results: true,
  maximized: null,
};
const storageKey = 'kessetsu.workspace-layout.v2';
const legacyStorageKey = 'kessetsu.workspace-layout.v1';
const panels: Panel[] = ['source', 'schematic', 'results'];
const icons = { source: Code2, schematic: CircuitBoard, results: Activity };
const labels = { source: 'Source', schematic: 'Schematic', results: 'Simulation' };
const clamp = (value: number) => Math.max(20, Math.min(80, value));

function readLayout(): LayoutState {
  try {
    const raw = localStorage.getItem(storageKey) ?? localStorage.getItem(legacyStorageKey);
    const saved = JSON.parse(raw ?? 'null');
    const maximized = saved?.maximized === null || panels.includes(saved?.maximized)
      ? saved.maximized as Panel | null
      : null;
    if (saved && panels.every((key) => typeof saved[key] === 'boolean')
      && Number.isFinite(saved.horizontal) && Number.isFinite(saved.vertical)) {
      return {
        ...defaults,
        ...saved,
        maximized,
        horizontal: clamp(saved.horizontal),
        vertical: clamp(saved.vertical),
      };
    }
  } catch { /* Storage is optional, including in private browsing. */ }
  return { ...defaults };
}

function Splitter({ axis, value, onChange }: { axis: 'horizontal' | 'vertical'; value: number; onChange(value: number): void }) {
  const dragging = useRef(false);
  return <div
    className={`workspace-splitter splitter-${axis}`}
    role="separator" tabIndex={0}
    aria-label={axis === 'horizontal' ? 'Resize source panel' : 'Resize schematic and results'}
    aria-orientation={axis === 'horizontal' ? 'vertical' : 'horizontal'}
    aria-valuemin={20} aria-valuemax={80} aria-valuenow={Math.round(value)}
    onPointerDown={(event) => {
      if (event.button !== 0) return;
      dragging.current = true;
      event.currentTarget.setPointerCapture(event.pointerId);
      event.preventDefault();
    }}
    onPointerMove={(event) => {
      if (!dragging.current) return;
      const rect = event.currentTarget.parentElement!.getBoundingClientRect();
      const position = axis === 'horizontal'
        ? (event.clientX - rect.left) / rect.width
        : (event.clientY - rect.top) / rect.height;
      onChange(clamp(position * 100));
    }}
    onPointerUp={(event) => {
      dragging.current = false;
      event.currentTarget.releasePointerCapture(event.pointerId);
    }}
    onLostPointerCapture={() => { dragging.current = false; }}
    onKeyDown={(event) => {
      const negative = axis === 'horizontal' ? 'ArrowLeft' : 'ArrowUp';
      const positive = axis === 'horizontal' ? 'ArrowRight' : 'ArrowDown';
      if (![negative, positive, 'Home', 'End'].includes(event.key)) return;
      event.preventDefault();
      onChange(event.key === 'Home' ? 20 : event.key === 'End' ? 80 : clamp(value + (event.key === positive ? 2 : -2)));
    }}
  />;
}

interface Props {
  source: PanelRenderer;
  schematic: PanelRenderer;
  results: PanelRenderer;
  resetRequest: number;
}

export function WorkspaceLayout({ source, schematic, results, resetRequest }: Props) {
  const [layout, setLayout] = useState(readLayout);
  const previousResetRequest = useRef(resetRequest);

  useEffect(() => {
    try { localStorage.setItem(storageKey, JSON.stringify(layout)); } catch { /* Nonessential preference. */ }
  }, [layout]);

  useEffect(() => {
    if (previousResetRequest.current !== resetRequest) {
      previousResetRequest.current = resetRequest;
      setLayout({ ...defaults });
    }
  }, [resetRequest]);

  useEffect(() => {
    const restore = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setLayout((current) => current.maximized ? { ...current, maximized: null } : current);
    };
    window.addEventListener('keydown', restore);
    return () => window.removeEventListener('keydown', restore);
  }, []);

  const isVisible = (panel: Panel) => layout[panel] && (!layout.maximized || layout.maximized === panel);
  const sourceVisible = isVisible('source');
  const schematicVisible = isVisible('schematic');
  const resultsVisible = isVisible('results');
  const rightVisible = schematicVisible || resultsVisible;
  const bothColumns = sourceVisible && rightVisible;
  const bothRows = schematicVisible && resultsVisible;
  const minimizedPanels = panels.filter((panel) => !layout[panel]);

  const controls = (panel: Panel): PanelWindowControls => ({
    panel,
    maximized: layout.maximized === panel,
    minimize: () => setLayout((current) => ({ ...current, [panel]: false, maximized: null })),
    toggleMaximize: () => setLayout((current) => ({
      ...current,
      [panel]: true,
      maximized: current.maximized === panel ? null : panel,
    })),
  });

  return <div className="workspace-layout">
    <div
      className={`resizable-workspace${layout.maximized ? ' has-maximized-panel' : ''}`}
      data-maximized-panel={layout.maximized ?? undefined}
      style={{ gridTemplateColumns: bothColumns ? `minmax(0, ${layout.horizontal}fr) 5px minmax(0, ${100 - layout.horizontal}fr)` : 'minmax(0, 1fr)' }}
    >
      <div className="panel-slot source-slot" hidden={!sourceVisible}>{source(controls('source'))}</div>
      {bothColumns && <Splitter axis="horizontal" value={layout.horizontal} onChange={(horizontal) => setLayout((current) => ({ ...current, horizontal }))} />}
      <div className="output-panels" hidden={!rightVisible} style={{ gridTemplateRows: bothRows ? `minmax(0, ${layout.vertical}fr) 5px minmax(0, ${100 - layout.vertical}fr)` : 'minmax(0, 1fr)' }}>
        <div className="panel-slot" hidden={!schematicVisible}>{schematic(controls('schematic'))}</div>
        {bothRows && <Splitter axis="vertical" value={layout.vertical} onChange={(vertical) => setLayout((current) => ({ ...current, vertical }))} />}
        <div className="panel-slot" hidden={!resultsVisible}>{results(controls('results'))}</div>
      </div>
      {!sourceVisible && !rightVisible && <div className="workspace-empty">All panels are minimized.</div>}
    </div>
    {minimizedPanels.length > 0 && <nav className="panel-dock" aria-label="Minimized panels">
      {minimizedPanels.map((panel) => {
        const Icon = icons[panel];
        return <button key={panel} aria-label={`Restore minimized ${labels[panel].toLowerCase()} panel`} onClick={() => setLayout((current) => ({ ...current, [panel]: true, maximized: null }))}>
          <Icon size={13} /><span>{labels[panel]}</span>
        </button>;
      })}
    </nav>}
  </div>;
}
