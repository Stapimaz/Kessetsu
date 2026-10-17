import { useEffect, useRef, useState, type ReactNode } from 'react';
import type { OutputCanvasMode, PanelWindowControls } from './PanelHeader';

type Panel = 'source' | 'schematic' | 'results';
type PanelRenderer = (controls: PanelWindowControls) => ReactNode;
interface LayoutState {
  horizontal: number;
  vertical: number;
  source: boolean;
  canvasMode: OutputCanvasMode;
  maximized: Panel | null;
}

export interface WorkspaceLayoutCommand {
  id: number;
  action: 'toggle-source' | 'schematic' | 'results' | 'split';
}

const defaults: LayoutState = {
  horizontal: 40,
  vertical: 55,
  source: true,
  canvasMode: 'schematic',
  maximized: null,
};
const storageKey = 'kessetsu.workspace-layout.v3';
const panels: Panel[] = ['source', 'schematic', 'results'];
const canvasModes: OutputCanvasMode[] = ['schematic', 'results', 'split'];
const clamp = (value: number) => Math.max(15, Math.min(85, value));

function readLayout(): LayoutState {
  try {
    const raw = localStorage.getItem(storageKey);
    const saved = JSON.parse(raw ?? 'null');
    const maximized = saved?.maximized === null || panels.includes(saved?.maximized)
      ? saved.maximized as Panel | null
      : null;
    const canvasMode = canvasModes.includes(saved?.canvasMode)
      ? saved.canvasMode as OutputCanvasMode
      : defaults.canvasMode;
    if (saved && typeof saved.source === 'boolean'
      && Number.isFinite(saved.horizontal) && Number.isFinite(saved.vertical)) {
      return {
        ...defaults,
        source: saved.source,
        canvasMode,
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
  const defaultValue = axis === 'horizontal' ? defaults.horizontal : defaults.vertical;
  return <div
    className={`workspace-splitter splitter-${axis}`}
    role="separator" tabIndex={0}
    title="Drag to resize · Double-click to reset"
    aria-label={axis === 'horizontal' ? 'Resize source panel' : 'Resize schematic and results'}
    aria-orientation={axis === 'horizontal' ? 'vertical' : 'horizontal'}
    aria-valuemin={15} aria-valuemax={85} aria-valuenow={Math.round(value)}
    onDoubleClick={() => onChange(defaultValue)}
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
      onChange(event.key === 'Home' ? 15 : event.key === 'End' ? 85 : clamp(value + (event.key === positive ? 2 : -2)));
    }}
  />;
}

interface Props {
  source: PanelRenderer;
  schematic: PanelRenderer;
  results: PanelRenderer;
  resetRequest: number;
  sourceRevealRequest: number;
  layoutCommand?: WorkspaceLayoutCommand | null;
}

export function WorkspaceLayout({ source, schematic, results, resetRequest, sourceRevealRequest, layoutCommand = null }: Props) {
  const [layout, setLayout] = useState(readLayout);
  const previousResetRequest = useRef(resetRequest);
  const previousSourceRevealRequest = useRef(sourceRevealRequest);
  const previousCommandId = useRef<number | null>(layoutCommand?.id ?? null);

  useEffect(() => {
    if (previousSourceRevealRequest.current !== sourceRevealRequest) {
      previousSourceRevealRequest.current = sourceRevealRequest;
      setLayout((current) => ({ ...current, source: true, maximized: null }));
    }
  }, [sourceRevealRequest]);

  useEffect(() => {
    if (!layoutCommand || previousCommandId.current === layoutCommand.id) return;
    previousCommandId.current = layoutCommand.id;
    setLayout((current) => {
      if (layoutCommand.action === 'toggle-source') {
        return { ...current, source: !current.source, maximized: null };
      }
      return { ...current, canvasMode: layoutCommand.action, maximized: null };
    });
  }, [layoutCommand]);

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
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setLayout((current) => current.maximized ? { ...current, maximized: null } : current);
        return;
      }
      if ((event.ctrlKey || event.metaKey) && !event.shiftKey && !event.altKey && event.key.toLowerCase() === 'b') {
        if (document.querySelector('dialog[open]')) return;
        event.preventDefault();
        setLayout((current) => ({ ...current, source: !current.source, maximized: null }));
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  const sourceVisible = layout.maximized ? layout.maximized === 'source' : layout.source;
  const schematicVisible = layout.maximized
    ? layout.maximized === 'schematic'
    : layout.canvasMode === 'schematic' || layout.canvasMode === 'split';
  const resultsVisible = layout.maximized
    ? layout.maximized === 'results'
    : layout.canvasMode === 'results' || layout.canvasMode === 'split';
  const rightVisible = schematicVisible || resultsVisible;
  const bothColumns = sourceVisible && rightVisible;
  const bothRows = schematicVisible && resultsVisible;

  const setCanvasMode = (canvasMode: OutputCanvasMode) => {
    setLayout((current) => ({ ...current, canvasMode, maximized: null }));
  };

  const toggleSource = () => setLayout((current) => ({ ...current, source: !current.source, maximized: null }));

  const controls = (panel: Panel): PanelWindowControls => ({
    panel,
    maximized: layout.maximized === panel,
    visible: panel === 'source' ? sourceVisible : panel === 'schematic' ? schematicVisible : resultsVisible,
    sourceVisible,
    toggleSource,
    canvasMode: layout.canvasMode,
    setCanvasMode,
    minimize: () => {
      if (panel === 'source') setLayout((current) => ({ ...current, source: false, maximized: null }));
    },
    toggleMaximize: () => setLayout((current) => ({
      ...current,
      ...(panel === 'source' ? { source: true } : {}),
      ...(panel === 'schematic' && current.canvasMode === 'results' ? { canvasMode: 'schematic' as const } : {}),
      ...(panel === 'results' && current.canvasMode === 'schematic' ? { canvasMode: 'results' as const } : {}),
      maximized: current.maximized === panel ? null : panel,
    })),
  });

  return <div className="workspace-layout">
    <div
      className={`resizable-workspace${layout.maximized ? ' has-maximized-panel' : ''}`}
      data-maximized-panel={layout.maximized ?? undefined}
      data-canvas-mode={layout.canvasMode}
      style={{ gridTemplateColumns: bothColumns ? `minmax(0, ${layout.horizontal}fr) 5px minmax(0, ${100 - layout.horizontal}fr)` : 'minmax(0, 1fr)' }}
    >
      <div className="panel-slot source-slot" hidden={!sourceVisible}>{source(controls('source'))}</div>
      {bothColumns && <Splitter axis="horizontal" value={layout.horizontal} onChange={(horizontal) => setLayout((current) => ({ ...current, horizontal }))} />}
      <div className="output-panels" hidden={!rightVisible} style={{ gridTemplateRows: bothRows ? `minmax(0, ${layout.vertical}fr) 5px minmax(0, ${100 - layout.vertical}fr)` : 'minmax(0, 1fr)' }}>
        <div className="panel-slot" hidden={!schematicVisible}>{schematic(controls('schematic'))}</div>
        {bothRows && <Splitter axis="vertical" value={layout.vertical} onChange={(vertical) => setLayout((current) => ({ ...current, vertical }))} />}
        <div className="panel-slot" hidden={!resultsVisible}>{results(controls('results'))}</div>
      </div>
    </div>
  </div>;
}
