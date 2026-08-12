import { Download, FileCode2 } from 'lucide-react';

interface Props {
  svg: string;
  kicad: string;
  spice: string;
}

function downloadText(filename: string, content: string, mimeType: string) {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
}

export function ArtifactBar({ svg, kicad, spice }: Props) {
  return (
    <aside className="artifact-bar" aria-label="Exports">
      <div className="header-title"><FileCode2 size={16} /><strong>Export</strong></div>
      <button disabled={!svg} onClick={() => downloadText('circuit.svg', svg, 'image/svg+xml')}><Download size={13} /> SVG</button>
      <button disabled={!kicad} onClick={() => downloadText('circuit.kicad_sch', kicad, 'text/plain')}><Download size={13} /> KiCad</button>
      <button disabled={!spice} onClick={() => downloadText('circuit.spice', spice, 'text/plain')}><Download size={13} /> SPICE</button>
      <details className="spice-details">
        <summary>Generated SPICE Netlist</summary>
        <pre>{spice || 'Geçerli devre bekleniyor…'}</pre>
      </details>
    </aside>
  );
}
