import { Maximize2, Minimize2, Minus, PanelLeftClose, PanelLeftOpen, Rows2 } from 'lucide-react';
import type { MouseEvent, ReactNode } from 'react';

export type OutputCanvasMode = 'split' | 'schematic' | 'results';

export interface PanelWindowControls {
  panel: 'source' | 'schematic' | 'results';
  maximized: boolean;
  visible: boolean;
  sourceVisible?: boolean;
  toggleSource?(): void;
  canvasMode?: OutputCanvasMode;
  setCanvasMode?(mode: OutputCanvasMode): void;
  minimize(): void;
  toggleMaximize(): void;
}

interface Props {
  controls: PanelWindowControls;
  icon: ReactNode;
  title: string;
  children?: ReactNode;
}

export function PanelHeader({ controls, icon, title, children }: Props) {
  const toggleFromHeader = (event: MouseEvent<HTMLElement>) => {
    if (!(event.target as Element).closest('button, a, summary, select')) controls.toggleMaximize();
  };

  const isOutputPanel = controls.panel === 'schematic' || controls.panel === 'results';
  const showSourceExpand = isOutputPanel
    && controls.sourceVisible === false
    && controls.toggleSource
    && (controls.panel === 'schematic' || controls.canvasMode === 'results');
  const showCanvasSwitch = isOutputPanel
    && controls.setCanvasMode
    && !controls.maximized
    && (controls.canvasMode !== 'split' || controls.panel === 'schematic');

  return (
    <header className="workspace-header" onDoubleClick={toggleFromHeader}>
      {showSourceExpand && (
        <button
          type="button"
          className="source-expand-pill"
          aria-label="Expand source panel"
          title="Show Source editor (Ctrl+B)"
          onClick={controls.toggleSource}
        >
          <PanelLeftOpen size={13} />
          <span>Source</span>
        </button>
      )}
      <div className="header-title">{icon}<strong>{title}</strong></div>
      {showCanvasSwitch && (
        <div className="canvas-mode-switch" role="group" aria-label="Right workspace view mode">
          <button
            type="button"
            className={`canvas-mode-button${controls.canvasMode === 'schematic' ? ' is-active' : ''}`}
            aria-label="Focus schematic view"
            aria-pressed={controls.canvasMode === 'schematic'}
            title="Full-height Schematic view"
            onClick={() => controls.setCanvasMode!('schematic')}
          >
            Schematic
          </button>
          <button
            type="button"
            className={`canvas-mode-button${controls.canvasMode === 'results' ? ' is-active' : ''}`}
            aria-label="Focus simulation view"
            aria-pressed={controls.canvasMode === 'results'}
            title="Full-height Simulation view"
            onClick={() => controls.setCanvasMode!('results')}
          >
            Simulation
          </button>
          <button
            type="button"
            className={`canvas-mode-button${controls.canvasMode === 'split' ? ' is-active' : ''}`}
            aria-label="Split schematic and simulation"
            aria-pressed={controls.canvasMode === 'split'}
            title="Show Schematic and Simulation together"
            onClick={() => controls.setCanvasMode!('split')}
          >
            <Rows2 size={11} />
            <span>Split</span>
          </button>
        </div>
      )}
      {children}
      <div className="panel-window-actions" aria-label={`${title} panel window controls`}>
        <button
          aria-label={`Minimize ${title.toLowerCase()} panel`}
          title={controls.panel === 'source' ? 'Collapse Source panel (Ctrl+B)' : 'Minimize panel'}
          onClick={controls.minimize}
        >
          {controls.panel === 'source' ? <PanelLeftClose size={14} /> : <Minus size={14} />}
        </button>
        <button
          aria-label={controls.maximized ? `Restore ${title.toLowerCase()} panel from full workspace` : `Maximize ${title.toLowerCase()} panel`}
          title={controls.maximized ? 'Restore panel' : 'Maximize panel'}
          onClick={controls.toggleMaximize}
        >
          {controls.maximized ? <Minimize2 size={13} /> : <Maximize2 size={13} />}
        </button>
      </div>
    </header>
  );
}
