import { Maximize2, Minimize2, Minus } from 'lucide-react';
import type { MouseEvent, ReactNode } from 'react';

export interface PanelWindowControls {
  panel: 'source' | 'schematic' | 'results';
  maximized: boolean;
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

  return (
    <header className="workspace-header" onDoubleClick={toggleFromHeader}>
      <div className="header-title">{icon}<strong>{title}</strong></div>
      {children}
      <div className="panel-window-actions" aria-label={`${title} panel window controls`}>
        <button aria-label={`Minimize ${controls.panel} panel`} title="Minimize panel" onClick={controls.minimize}>
          <Minus size={14} />
        </button>
        <button
          aria-label={controls.maximized ? `Restore ${controls.panel} panel from full workspace` : `Maximize ${controls.panel} panel`}
          title={controls.maximized ? 'Restore panel' : 'Maximize panel'}
          onClick={controls.toggleMaximize}
        >
          {controls.maximized ? <Minimize2 size={13} /> : <Maximize2 size={13} />}
        </button>
      </div>
    </header>
  );
}
