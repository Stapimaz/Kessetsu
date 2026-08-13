import { CircuitBoard, Grid3X3, Maximize2, Minus, Plus } from 'lucide-react';
import { useState } from 'react';
import type { SchematicSummary } from '../domain';

interface Props {
  schematic: SchematicSummary | null;
  svg: string;
}

export function SchematicPanel({ schematic, svg }: Props) {
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [dragStart, setDragStart] = useState<{ x: number; y: number } | null>(null);
  const [gridVisible, setGridVisible] = useState(true);
  const reset = () => { setZoom(1); setPan({ x: 0, y: 0 }); };

  return (
    <section className="workspace-panel schematic-panel" aria-label="Canonical schematic">
      <header className="workspace-header">
        <div className="header-title"><CircuitBoard size={18} /><strong>Schematic</strong></div>
        <span className={`quality-badge ${schematic?.quality.passed ? 'quality-pass' : ''}`}>
          {schematic?.connectivity.verified ? 'Connectivity verified' : 'Waiting'}
        </span>
        <div className="icon-actions" aria-label="Schematic view">
          <button
            aria-label="Toggle schematic grid"
            aria-pressed={gridVisible}
            onClick={() => setGridVisible((current) => !current)}
          ><Grid3X3 size={14} /></button>
          <button aria-label="Zoom out" onClick={() => setZoom((current) => Math.max(0.2, current - 0.2))}><Minus size={14} /></button>
          <button aria-label="Fit view" onClick={reset}><Maximize2 size={14} /></button>
          <button aria-label="Zoom in" onClick={() => setZoom((current) => Math.min(4, current + 0.2))}><Plus size={14} /></button>
        </div>
      </header>
      <div
        className={`schematic-surface${gridVisible ? ' has-grid' : ''}${dragStart ? ' is-dragging' : ''}`}
        tabIndex={0}
        onWheel={(event) => {
          event.preventDefault();
          setZoom((current) => Math.max(0.2, Math.min(4, current - event.deltaY * 0.002)));
        }}
        onPointerDown={(event) => {
          event.currentTarget.setPointerCapture(event.pointerId);
          setDragStart({ x: event.clientX - pan.x, y: event.clientY - pan.y });
        }}
        onPointerMove={(event) => dragStart && setPan({ x: event.clientX - dragStart.x, y: event.clientY - dragStart.y })}
        onPointerUp={() => setDragStart(null)}
        onKeyDown={(event) => {
          const delta = event.shiftKey ? 40 : 12;
          if (event.key === 'ArrowLeft') setPan((current) => ({ ...current, x: current.x - delta }));
          if (event.key === 'ArrowRight') setPan((current) => ({ ...current, x: current.x + delta }));
          if (event.key === 'ArrowUp') setPan((current) => ({ ...current, y: current.y - delta }));
          if (event.key === 'ArrowDown') setPan((current) => ({ ...current, y: current.y + delta }));
        }}
      >
        {svg ? (
          <div
            className="schematic-document"
            data-testid="canonical-schematic"
            data-quality={schematic?.quality.passed ? 'pass' : 'warning'}
            style={{ transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})` }}
            dangerouslySetInnerHTML={{ __html: svg }}
          />
        ) : <div className="empty-state">Geçerli devre bekleniyor…</div>}
      </div>
    </section>
  );
}
