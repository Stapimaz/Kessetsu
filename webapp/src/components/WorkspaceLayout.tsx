import { useEffect, useRef, useState, type ReactNode } from 'react';

const defaults = { horizontal: 40, vertical: 55, source: true, schematic: true, results: true };
const storageKey = 'kessetsu.workspace-layout.v1';
type Panel = 'source' | 'schematic' | 'results';
const panels: Panel[] = ['source', 'schematic', 'results'];
const clamp = (value: number) => Math.max(20, Math.min(80, value));

function readLayout() {
  try {
    const saved = JSON.parse(localStorage.getItem(storageKey) ?? 'null');
    if (saved && panels.every((key) => typeof saved[key] === 'boolean')
      && panels.some((key) => saved[key])
      && Number.isFinite(saved.horizontal) && Number.isFinite(saved.vertical)) {
      return { ...defaults, ...saved, horizontal: clamp(saved.horizontal), vertical: clamp(saved.vertical) } as typeof defaults;
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
      const position = axis === 'horizontal' ? (event.clientX - rect.left) / rect.width : (event.clientY - rect.top) / rect.height;
      onChange(clamp(position * 100));
    }}
    onPointerUp={(event) => { dragging.current = false; event.currentTarget.releasePointerCapture(event.pointerId); }}
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

export function WorkspaceLayout({ source, schematic, results }: Record<Panel, ReactNode>) {
  const [layout, setLayout] = useState(readLayout);
  useEffect(() => {
    try { localStorage.setItem(storageKey, JSON.stringify(layout)); } catch { /* Nonessential preference. */ }
  }, [layout]);
  const rightVisible = layout.schematic || layout.results;
  const bothColumns = layout.source && rightVisible;
  const bothRows = layout.schematic && layout.results;

  return <div className="workspace-layout">
    <nav className="panel-toolbar" aria-label="Workspace panels">
      <span>Panels</span>
      {panels.map((panel) => <button key={panel} aria-pressed={layout[panel]}
        aria-label={`${layout[panel] ? 'Minimize' : 'Restore'} ${panel} panel`}
        disabled={layout[panel] && panels.filter((key) => layout[key]).length === 1}
        onClick={() => setLayout((current) => ({ ...current, [panel]: !current[panel] }))}
      >{panel}</button>)}
      <button className="reset-layout" onClick={() => setLayout({ ...defaults })}>Reset layout</button>
    </nav>
    <div className="resizable-workspace" style={{ gridTemplateColumns: bothColumns ? `minmax(0, ${layout.horizontal}fr) 6px minmax(0, ${100 - layout.horizontal}fr)` : 'minmax(0, 1fr)' }}>
      <div className="panel-slot source-slot" hidden={!layout.source}>{source}</div>
      {bothColumns && <Splitter axis="horizontal" value={layout.horizontal} onChange={(horizontal) => setLayout((current) => ({ ...current, horizontal }))} />}
      <div className="output-panels" hidden={!rightVisible} style={{ gridTemplateRows: bothRows ? `minmax(0, ${layout.vertical}fr) 6px minmax(0, ${100 - layout.vertical}fr)` : 'minmax(0, 1fr)' }}>
        <div className="panel-slot" hidden={!layout.schematic}>{schematic}</div>
        {bothRows && <Splitter axis="vertical" value={layout.vertical} onChange={(vertical) => setLayout((current) => ({ ...current, vertical }))} />}
        <div className="panel-slot" hidden={!layout.results}>{results}</div>
      </div>
    </div>
  </div>;
}
