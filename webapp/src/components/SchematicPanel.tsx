import { CircuitBoard, Grid3X3, Maximize2, Minus, Plus } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import type { SchematicSummary } from '../domain';

interface Props {
  schematic: SchematicSummary | null;
  svg: string;
}

function componentAtTarget(target: EventTarget | null) {
  return target instanceof Element
    ? target.closest('[data-component]')?.getAttribute('data-component') ?? null
    : null;
}

function componentAtPointer(
  target: EventTarget | null,
  surface: HTMLDivElement,
  clientX: number,
  clientY: number,
) {
  const direct = componentAtTarget(target);
  if (direct) return direct;

  const padding = 6;
  const matches = Array.from(
    surface.querySelectorAll<SVGGElement>('g.component[data-component]'),
  ).flatMap((group) => {
    const bounds = group.getBoundingClientRect();
    if (
      clientX < bounds.left - padding
      || clientX > bounds.right + padding
      || clientY < bounds.top - padding
      || clientY > bounds.bottom + padding
    ) return [];
    return [{
      component: group.getAttribute('data-component') ?? '',
      area: bounds.width * bounds.height,
    }];
  });
  matches.sort((left, right) => left.area - right.area || left.component.localeCompare(right.component));
  return matches[0]?.component || null;
}

export function SchematicPanel({ schematic, svg }: Props) {
  const [view, setView] = useState({ zoom: 1, pan: { x: 0, y: 0 } });
  const [dragStart, setDragStart] = useState<{ x: number; y: number } | null>(null);
  const [gridVisible, setGridVisible] = useState(true);
  const [hoveredComponent, setHoveredComponent] = useState<string | null>(null);
  const [selectedComponent, setSelectedComponent] = useState<string | null>(null);
  const documentRef = useRef<HTMLDivElement>(null);
  const { zoom, pan } = view;
  const activeComponent = hoveredComponent ?? selectedComponent;
  const reset = () => setView({ zoom: 1, pan: { x: 0, y: 0 } });

  useEffect(() => {
    setHoveredComponent(null);
    setSelectedComponent(null);
  }, [svg]);

  useEffect(() => {
    const nodes = documentRef.current?.querySelectorAll('[data-component]') ?? [];
    for (const node of nodes) {
      node.classList.toggle(
        'is-component-active',
        activeComponent !== null && node.getAttribute('data-component') === activeComponent,
      );
    }
  }, [activeComponent, svg]);

  useEffect(() => {
    const clearSelection = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setSelectedComponent(null);
    };
    window.addEventListener('keydown', clearSelection);
    return () => window.removeEventListener('keydown', clearSelection);
  }, []);

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
          <button aria-label="Zoom out" onClick={() => setView((current) => ({ ...current, zoom: Math.max(0.2, current.zoom - 0.2) }))}><Minus size={14} /></button>
          <button aria-label="Fit view" onClick={reset}><Maximize2 size={14} /></button>
          <button aria-label="Zoom in" onClick={() => setView((current) => ({ ...current, zoom: Math.min(4, current.zoom + 0.2) }))}><Plus size={14} /></button>
        </div>
      </header>
      <div
        className={`schematic-surface${gridVisible ? ' has-grid' : ''}${dragStart ? ' is-dragging' : ''}`}
        tabIndex={0}
        onWheel={(event) => {
          event.preventDefault();
          const bounds = event.currentTarget.getBoundingClientRect();
          const anchor = {
            x: event.clientX - bounds.left - bounds.width / 2,
            y: event.clientY - bounds.top - bounds.height / 2,
          };
          setView((current) => {
            const nextZoom = Math.max(0.2, Math.min(4, current.zoom * Math.exp(-event.deltaY * 0.0015)));
            const ratio = nextZoom / current.zoom;
            return {
              zoom: nextZoom,
              pan: {
                x: anchor.x - (anchor.x - current.pan.x) * ratio,
                y: anchor.y - (anchor.y - current.pan.y) * ratio,
              },
            };
          });
        }}
        onPointerDown={(event) => {
          event.currentTarget.focus();
          const component = componentAtPointer(
            event.target,
            event.currentTarget,
            event.clientX,
            event.clientY,
          );
          if (component) {
            setSelectedComponent((current) => component === current ? null : component);
            return;
          }
          setSelectedComponent(null);
          event.currentTarget.setPointerCapture(event.pointerId);
          setDragStart({ x: event.clientX - pan.x, y: event.clientY - pan.y });
        }}
        onPointerMove={(event) => {
          setHoveredComponent(componentAtPointer(
            event.target,
            event.currentTarget,
            event.clientX,
            event.clientY,
          ));
          if (dragStart) {
            setView((current) => ({
              ...current,
              pan: { x: event.clientX - dragStart.x, y: event.clientY - dragStart.y },
            }));
          }
        }}
        onPointerUp={() => setDragStart(null)}
        onPointerLeave={() => { setHoveredComponent(null); setDragStart(null); }}
        onKeyDown={(event) => {
          const delta = event.shiftKey ? 40 : 12;
          if (event.key === 'Escape') setSelectedComponent(null);
          if (event.key === 'ArrowLeft') setView((current) => ({ ...current, pan: { ...current.pan, x: current.pan.x - delta } }));
          if (event.key === 'ArrowRight') setView((current) => ({ ...current, pan: { ...current.pan, x: current.pan.x + delta } }));
          if (event.key === 'ArrowUp') setView((current) => ({ ...current, pan: { ...current.pan, y: current.pan.y - delta } }));
          if (event.key === 'ArrowDown') setView((current) => ({ ...current, pan: { ...current.pan, y: current.pan.y + delta } }));
        }}
      >
        {svg ? (
          <div
            ref={documentRef}
            className="schematic-document"
            data-testid="canonical-schematic"
            data-quality={schematic?.quality.passed ? 'pass' : 'warning'}
            data-selected-component={selectedComponent ?? undefined}
            style={{ transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})` }}
            dangerouslySetInnerHTML={{ __html: svg }}
          />
        ) : <div className="empty-state">Geçerli devre bekleniyor…</div>}
      </div>
    </section>
  );
}
