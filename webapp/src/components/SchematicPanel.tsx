import { Boxes, CircuitBoard, Focus, Grid3X3, Minus, Plus, Search, X } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import type { CircuitIrSummary, SchematicComponent, SchematicSummary } from '../domain';
import { PanelHeader, type PanelWindowControls } from './PanelHeader';

interface Props {
  schematic: SchematicSummary | null;
  circuitIr: CircuitIrSummary | null;
  svg: string;
  panelControls: PanelWindowControls;
}

type Selection =
  | { type: 'component'; id: string }
  | { type: 'net'; id: number }
  | { type: 'group'; path: string[] };

function componentAtTarget(target: EventTarget | null) {
  return target instanceof Element
    ? target.closest('[data-component]')?.getAttribute('data-component') ?? null
    : null;
}

function netAtTarget(target: EventTarget | null) {
  if (!(target instanceof Element)) return null;
  const value = target.closest('[data-net]')?.getAttribute('data-net');
  if (value === null || value === undefined || !/^\d+$/.test(value)) return null;
  return Number(value);
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

function pathKey(path: string[]) {
  return path.join('\u001f');
}

function pathStartsWith(path: string[], prefix: string[]) {
  return prefix.every((part, index) => path[index] === part);
}

function componentKind(kind: string | Record<string, unknown> | undefined) {
  if (!kind) return 'Component';
  if (typeof kind === 'string') return kind.replaceAll('_', ' ');
  return Object.keys(kind)[0]?.replaceAll('_', ' ') ?? 'Component';
}

export function SchematicPanel({ schematic, circuitIr, svg, panelControls }: Props) {
  const [view, setView] = useState({ zoom: 1, pan: { x: 0, y: 0 } });
  const [dragStart, setDragStart] = useState<{ x: number; y: number } | null>(null);
  const [gridVisible, setGridVisible] = useState(true);
  const [hoveredComponent, setHoveredComponent] = useState<string | null>(null);
  const [selection, setSelection] = useState<Selection | null>(null);
  const [navigatorOpen, setNavigatorOpen] = useState(false);
  const [query, setQuery] = useState('');
  const documentRef = useRef<HTMLDivElement>(null);
  const { zoom, pan } = view;
  const activeComponent = hoveredComponent ?? (selection?.type === 'component' ? selection.id : null);
  const reset = () => setView({ zoom: 1, pan: { x: 0, y: 0 } });

  const irComponents = useMemo(() => new Map(
    (circuitIr?.components ?? []).map((component) => [component.id, component]),
  ), [circuitIr]);
  const components = useMemo(() => schematic?.components ?? [], [schematic]);
  const nets = useMemo(() => schematic?.nets ?? [], [schematic]);
  const groups = useMemo(() => {
    const unique = new Map<string, string[]>();
    for (const component of circuitIr?.components ?? []) {
      const path = component.instance_path ?? [];
      for (let depth = 1; depth <= path.length; depth += 1) {
        const group = path.slice(0, depth);
        unique.set(pathKey(group), group);
      }
    }
    return [...unique.values()].sort((left, right) => pathKey(left).localeCompare(pathKey(right)));
  }, [circuitIr]);

  const selectedComponent = selection?.type === 'component'
    ? components.find((component) => component.id === selection.id) ?? null
    : null;
  const selectedNet = selection?.type === 'net'
    ? nets.find((net) => net.id === selection.id) ?? null
    : null;

  const focusSelector = (selector: string) => {
    globalThis.requestAnimationFrame(() => {
      const surface = documentRef.current?.parentElement;
      const matches = [...(documentRef.current?.querySelectorAll<SVGGraphicsElement>(selector) ?? [])];
      if (!surface || matches.length === 0) return;
      const boxes = matches.map((element) => element.getBoundingClientRect());
      const left = Math.min(...boxes.map((box) => box.left));
      const right = Math.max(...boxes.map((box) => box.right));
      const top = Math.min(...boxes.map((box) => box.top));
      const bottom = Math.max(...boxes.map((box) => box.bottom));
      const viewport = surface.getBoundingClientRect();
      setView((current) => ({
        ...current,
        pan: {
          x: current.pan.x + viewport.left + viewport.width / 2 - (left + right) / 2,
          y: current.pan.y + viewport.top + viewport.height / 2 - (top + bottom) / 2,
        },
      }));
    });
  };

  const chooseComponent = (id: string, focus = false) => {
    setSelection({ type: 'component', id });
    setNavigatorOpen(true);
    if (focus) focusSelector(`[data-component="${CSS.escape(id)}"]`);
  };
  const chooseNet = (id: number, focus = false) => {
    setSelection({ type: 'net', id });
    setNavigatorOpen(true);
    if (focus) focusSelector(`[data-net="${id}"]`);
  };
  const chooseGroup = (path: string[]) => {
    setSelection({ type: 'group', path });
    setNavigatorOpen(true);
    globalThis.requestAnimationFrame(() => focusSelector('.is-group-active'));
  };

  useEffect(() => {
    setHoveredComponent(null);
    setSelection(null);
    setNavigatorOpen(false);
    setQuery('');
  }, [svg]);

  useEffect(() => {
    const root = documentRef.current;
    if (!root) return;
    const selectedGroup = selection?.type === 'group' ? selection.path : null;
    const connected = new Set(
      selection?.type === 'net'
        ? nets.find((net) => net.id === selection.id)?.pins.map((pin) => pin.component) ?? []
        : [],
    );
    root.classList.toggle('has-active-selection', selection !== null);
    for (const node of root.querySelectorAll('[data-component]')) {
      const id = node.getAttribute('data-component') ?? '';
      const instancePath = irComponents.get(id)?.instance_path ?? [];
      node.classList.toggle('is-component-active', activeComponent === id);
      node.classList.toggle('is-net-connected', connected.has(id));
      node.classList.toggle('is-group-active', selectedGroup !== null && pathStartsWith(instancePath, selectedGroup));
    }
    for (const node of root.querySelectorAll('[data-net]')) {
      node.classList.toggle(
        'is-net-active',
        selection?.type === 'net' && node.getAttribute('data-net') === String(selection.id),
      );
    }
  }, [activeComponent, irComponents, nets, selection, svg]);

  useEffect(() => {
    const clearSelection = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setSelection(null);
        setNavigatorOpen(false);
      }
    };
    window.addEventListener('keydown', clearSelection);
    return () => window.removeEventListener('keydown', clearSelection);
  }, []);

  const normalizedQuery = query.trim().toLocaleLowerCase('en-US');
  const matchingComponents = components.filter((component) => {
    const ir = irComponents.get(component.id);
    return [component.reference, component.id, component.value, component.model,
      ...(ir?.instance_path ?? [])].filter(Boolean).join(' ').toLocaleLowerCase('en-US').includes(normalizedQuery);
  }).slice(0, 40);
  const matchingNets = nets.filter((net) => net.name.toLocaleLowerCase('en-US').includes(normalizedQuery)).slice(0, 40);

  const componentRow = (component: SchematicComponent) => {
    const ir = irComponents.get(component.id);
    return <button key={component.id} className="navigator-result" onClick={() => chooseComponent(component.id, true)}>
      <span><strong>{component.reference}</strong><small>{component.value ?? component.model ?? componentKind(ir?.kind)}</small></span>
      {(ir?.instance_path?.length ?? 0) > 0 && <small>{ir!.instance_path!.join(' / ')}</small>}
    </button>;
  };

  return (
    <section className="workspace-panel schematic-panel" aria-label="Canonical schematic">
      <PanelHeader controls={panelControls} icon={<CircuitBoard size={15} />} title="Schematic">
        <span className={`quality-badge ${schematic?.quality.passed ? 'quality-pass' : ''}`}>
          {schematic?.connectivity.verified ? 'Connectivity verified' : 'Waiting'}
        </span>
        <div className="icon-actions" aria-label="Schematic view">
          <button
            aria-label="Find and inspect schematic"
            aria-pressed={navigatorOpen}
            title="Find components and nets"
            onClick={() => setNavigatorOpen((current) => !current)}
          ><Search size={14} /></button>
          <button
            aria-label="Toggle schematic grid"
            aria-pressed={gridVisible}
            onClick={() => setGridVisible((current) => !current)}
          ><Grid3X3 size={14} /></button>
          <button aria-label="Zoom out" onClick={() => setView((current) => ({ ...current, zoom: Math.max(0.2, current.zoom - 0.2) }))}><Minus size={14} /></button>
          <button aria-label="Fit view" onClick={reset}><Focus size={14} /></button>
          <button aria-label="Zoom in" onClick={() => setView((current) => ({ ...current, zoom: Math.min(4, current.zoom + 0.2) }))}><Plus size={14} /></button>
        </div>
      </PanelHeader>
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
          const directNet = netAtTarget(event.target);
          const component = componentAtTarget(event.target)
            ?? (directNet === null
              ? componentAtPointer(event.target, event.currentTarget, event.clientX, event.clientY)
              : null);
          if (component) {
            setSelection((current) => current?.type === 'component' && current.id === component ? null : { type: 'component', id: component });
            setNavigatorOpen(true);
            return;
          }
          if (directNet !== null) {
            setSelection((current) => current?.type === 'net' && current.id === directNet ? null : { type: 'net', id: directNet });
            setNavigatorOpen(true);
            return;
          }
          setSelection(null);
          event.currentTarget.setPointerCapture(event.pointerId);
          setDragStart({ x: event.clientX - pan.x, y: event.clientY - pan.y });
        }}
        onPointerMove={(event) => {
          setHoveredComponent(componentAtPointer(event.target, event.currentTarget, event.clientX, event.clientY));
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
          if (event.key === 'Escape') setSelection(null);
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
            data-selected-component={selection?.type === 'component' ? selection.id : undefined}
            data-selected-net={selection?.type === 'net' ? selection.id : undefined}
            style={{ transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})` }}
            dangerouslySetInnerHTML={{ __html: svg }}
          />
        ) : <div className="empty-state">Waiting for a valid circuit…</div>}

        {svg && navigatorOpen && <aside className="schematic-navigator" aria-label="Schematic inspector" onPointerDown={(event) => event.stopPropagation()} onWheel={(event) => event.stopPropagation()} onKeyDown={(event) => { if (event.key.startsWith('Arrow')) event.stopPropagation(); }}>
          <header><div><strong>{selection ? 'Inspect' : 'Navigate'}</strong><small>{components.length} components · {nets.length} nets</small></div><button aria-label="Close schematic inspector" onClick={() => setNavigatorOpen(false)}><X size={14} /></button></header>
          <label className="navigator-search"><Search size={13} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Find component or net" /></label>

          {selectedComponent && <div className="inspector-content">
            <div className="inspector-heading"><strong>{selectedComponent.reference}</strong><span>{componentKind(irComponents.get(selectedComponent.id)?.kind)}</span></div>
            {irComponents.get(selectedComponent.id)?.instance_path?.length
              ? <button className="group-breadcrumb" onClick={() => chooseGroup(irComponents.get(selectedComponent.id)!.instance_path!)}><Boxes size={12} />{irComponents.get(selectedComponent.id)!.instance_path!.join(' / ')}</button>
              : <small>Top-level circuit</small>}
            <dl>
              {selectedComponent.value && <><dt>Value</dt><dd>{selectedComponent.value}</dd></>}
              {selectedComponent.model && <><dt>Model</dt><dd>{selectedComponent.model}</dd></>}
              {Object.entries(selectedComponent.instance_parameters ?? {}).map(([name, value]) => <span className="inspector-pair" key={name}><dt>{name}</dt><dd>{value}</dd></span>)}
            </dl>
            <h4>Pins</h4>
            <div className="inspector-pin-list">{selectedComponent.pins.map((pin) => {
              const net = nets.find((candidate) => candidate.id === pin.net);
              return <button key={pin.name} disabled={!net} onClick={() => net && chooseNet(net.id, true)}><span>{pin.name}</span><strong>{net?.name ?? 'unconnected'}</strong></button>;
            })}</div>
            {selectedComponent.model_metadata && <details><summary>Model provenance</summary><dl><dt>Source</dt><dd>{selectedComponent.model_metadata.source}</dd><dt>Version</dt><dd>{selectedComponent.model_metadata.version}</dd><dt>Simulator</dt><dd>{selectedComponent.model_metadata.simulator}</dd><dt>Hash</dt><dd className="inspector-hash">{selectedComponent.model_metadata.content_hash}</dd></dl></details>}
          </div>}

          {selectedNet && <div className="inspector-content">
            <div className="inspector-heading"><strong>{selectedNet.name}</strong><span>{selectedNet.kind} net</span></div>
            <h4>Connected pins</h4>
            <div className="inspector-pin-list">{selectedNet.pins.map((pin) => <button key={`${pin.component}.${pin.pin}`} onClick={() => chooseComponent(pin.component, true)}><span>{pin.pin}</span><strong>{pin.component}</strong></button>)}</div>
          </div>}

          {selection?.type === 'group' && <div className="inspector-content">
            <div className="inspector-heading"><strong>{selection.path.at(-1)}</strong><span>Reusable block</span></div>
            <small>{selection.path.join(' / ')}</small>
            <h4>Components</h4>
            <div className="navigator-results">{components.filter((component) => pathStartsWith(irComponents.get(component.id)?.instance_path ?? [], selection.path)).map(componentRow)}</div>
          </div>}

          {!selection && !normalizedQuery && <div className="navigator-section">
            {groups.length > 0 && <><h4>Reusable blocks</h4>{groups.map((path) => <button className="navigator-result" key={pathKey(path)} onClick={() => chooseGroup(path)}><span><strong>{path.at(-1)}</strong><small>{components.filter((component) => pathStartsWith(irComponents.get(component.id)?.instance_path ?? [], path)).length} components</small></span><small>{path.slice(0, -1).join(' / ')}</small></button>)}</>}
            <h4>Components</h4>
            <div className="navigator-results">{components.slice(0, 12).map(componentRow)}{components.length > 12 && <small>Search to inspect {components.length - 12} more components.</small>}</div>
            <h4>Nets</h4>
            <div className="navigator-results">{nets.slice(0, 12).map((net) => <button className="navigator-result" key={`net-${net.id}`} onClick={() => chooseNet(net.id, true)}><span><strong>{net.name}</strong><small>{net.kind} · {net.pins.length} pins</small></span></button>)}{nets.length > 12 && <small>Search to inspect {nets.length - 12} more nets.</small>}</div>
          </div>}
          {normalizedQuery && <div className="navigator-section"><h4>Search results</h4><div className="navigator-results">{matchingComponents.map(componentRow)}{matchingNets.map((net) => <button className="navigator-result" key={`net-${net.id}`} onClick={() => chooseNet(net.id, true)}><span><strong>{net.name}</strong><small>{net.kind} · {net.pins.length} pins</small></span></button>)}{matchingComponents.length + matchingNets.length === 0 && <small>No matching component or net.</small>}</div></div>}
        </aside>}
      </div>
    </section>
  );
}
