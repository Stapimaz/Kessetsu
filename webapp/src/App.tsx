import React, { useCallback, useEffect, useState } from 'react';
import Editor from '@monaco-editor/react';
import { Play, Code2, CircuitBoard, Terminal as TerminalIcon, Download } from 'lucide-react';
import init, { compile_netlang } from 'netlang-core';
import defaultCircuit from '../../examples/demo_circuit.nl?raw';

const COMPILE_SCHEMA_VERSION = 'netlang.compile.v1';

interface CompileDiagnostic {
  code: string;
  severity: 'error' | 'warning' | 'info';
  stage: 'parse' | 'flatten' | 'semantic' | 'erc' | 'io' | 'cli' | 'simulation' | 'assertion';
  message: string;
}

interface ComponentPosition {
  x: number;
  y: number;
  comp_type: string;
  width: number;
  height: number;
  rotation: number;
}

interface LayoutWire {
  net_id: number;
  points: [number, number][];
}

interface LayoutResult {
  components: Record<string, ComponentPosition>;
  wires: LayoutWire[];
}

interface CompileReport {
  schema_version: string;
  diagnostics: CompileDiagnostic[];
  layout: LayoutResult | null;
  kicad_sch: string | null;
  spice_netlist: string | null;
}

interface UiDiagnostic {
  message: string;
  type: CompileDiagnostic['severity'];
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function App() {
  const [code, setCode] = useState(defaultCircuit);
  const [errors, setErrors] = useState<UiDiagnostic[]>([]);
  const [success, setSuccess] = useState<boolean>(false);
  const [isWasmLoaded, setIsWasmLoaded] = useState(false);
  const [layout, setLayout] = useState<LayoutResult | null>(null);
  const [kicadSch, setKicadSch] = useState<string>('');

  const [spiceNetlist, setSpiceNetlist] = useState<string>('');

  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });

  useEffect(() => {
    init().then(() => {
      setIsWasmLoaded(true);
    });
  }, []);

  const compileCode = useCallback(() => {
    if (!isWasmLoaded) return;
    
    try {
      const result = compile_netlang(code) as CompileReport;
      if (result.schema_version !== COMPILE_SCHEMA_VERSION) {
        throw new Error(`Unsupported compile report schema: ${result.schema_version}`);
      }

      const diagnostics = result.diagnostics || [];
      const hasErrors = diagnostics.some((diagnostic) => diagnostic.severity === 'error');
      setErrors(diagnostics.map((diagnostic) => ({
        message: `[${diagnostic.code}/${diagnostic.stage}] ${diagnostic.message}`,
        type: diagnostic.severity,
      })));

      if (hasErrors) {
        setSuccess(false);
        setSpiceNetlist('');
        setKicadSch('');
        setLayout(null);
      } else {
        setSuccess(true);
        setLayout(result.layout);
        setSpiceNetlist(result.spice_netlist || '');
        setKicadSch(result.kicad_sch || '');
      }
    } catch (error: unknown) {
      setErrors([{ message: `WASM Execution Error: ${errorMessage(error)}`, type: 'error' }]);
      setSuccess(false);
      setSpiceNetlist('');
      setKicadSch('');
      setLayout(null);
    }
  }, [code, isWasmLoaded]);

  useEffect(() => {
    compileCode();
  }, [compileCode]);

  // Schematic Renderer using Rust Auto-Layout
  const renderCircuit = () => {
    if (!layout || !layout.components) return <div style={{color: '#94a3b8'}}>Şema hesaplanıyor...</div>;
    
    const SCALE = 40; // 1 Grid Unit = 40px
    const OFFSET_X = 80;
    const OFFSET_Y = 100;

    const renderSymbol = (type: string) => {
      switch(type) {
        case 'Resistor':
          return (
            <g>
              <line x1="0" y1="0" x2="15" y2="0" stroke="var(--accent)" strokeWidth="2" />
              <polyline points="15,0 20,-10 30,10 40,-10 50,10 60,-10 65,0" fill="none" stroke="var(--accent)" strokeWidth="2" strokeLinejoin="bevel" />
              <line x1="65" y1="0" x2="80" y2="0" stroke="var(--accent)" strokeWidth="2" />
            </g>
          );
        case 'Source':
          return (
            <g>
              <line x1="0" y1="0" x2="20" y2="0" stroke="var(--accent)" strokeWidth="2" />
              <circle cx="40" cy="0" r="20" fill="var(--panel-bg)" stroke="var(--accent)" strokeWidth="2" />
              <text x="40" y="-5" fill="var(--accent)" fontSize="14" textAnchor="middle">+</text>
              <text x="40" y="14" fill="var(--accent)" fontSize="14" textAnchor="middle">−</text>
              <line x1="60" y1="0" x2="80" y2="0" stroke="var(--accent)" strokeWidth="2" />
            </g>
          );
        case 'CurrentSource':
          return (
            <g>
              <line x1="0" y1="0" x2="20" y2="0" stroke="var(--accent)" strokeWidth="2" />
              <circle cx="40" cy="0" r="20" fill="var(--panel-bg)" stroke="var(--accent)" strokeWidth="2" />
              <line x1="30" y1="0" x2="50" y2="0" stroke="var(--accent)" strokeWidth="2" />
              <polygon points="50,0 42,-5 42,5" fill="var(--accent)" />
              <line x1="60" y1="0" x2="80" y2="0" stroke="var(--accent)" strokeWidth="2" />
            </g>
          );
        case 'Inductor':
          return (
            <g>
              <path d="M 0 0 C 15 -20 25 -20 20 0 C 35 -20 45 -20 40 0 C 55 -20 65 -20 60 0 C 75 -20 80 -20 80 0" fill="none" stroke="var(--accent)" strokeWidth="2" strokeLinecap="round" />
            </g>
          );
        case 'Diode':
          return (
            <g>
              <line x1="0" y1="0" x2="30" y2="0" stroke="var(--accent)" strokeWidth="2" />
              <polygon points="30,-10 50,0 30,10" fill="none" stroke="var(--accent)" strokeWidth="2" />
              <line x1="50" y1="-10" x2="50" y2="10" stroke="var(--accent)" strokeWidth="2" />
              <line x1="50" y1="0" x2="80" y2="0" stroke="var(--accent)" strokeWidth="2" />
            </g>
          );
        case 'Transistor':
          return (
            <g>
              {/* Base */}
              <line x1="0" y1="40" x2="30" y2="40" stroke="var(--accent)" strokeWidth="2" />
              <line x1="30" y1="20" x2="30" y2="60" stroke="var(--accent)" strokeWidth="3" />
              {/* Collector */}
              <line x1="30" y1="30" x2="80" y2="0" stroke="var(--accent)" strokeWidth="2" />
              {/* Emitter */}
              <line x1="30" y1="50" x2="80" y2="80" stroke="var(--accent)" strokeWidth="2" />
              <polygon points="65,71 80,80 71,65" fill="var(--accent)" />
              <circle cx="50" cy="40" r="35" fill="none" stroke="var(--accent)" strokeWidth="1" strokeDasharray="2" />
            </g>
          );
        case 'Mosfet':
          return (
            <g>
              <line x1="0" y1="40" x2="25" y2="40" stroke="var(--accent)" strokeWidth="2" />
              <line x1="25" y1="20" x2="25" y2="60" stroke="var(--accent)" strokeWidth="3" />
              <line x1="35" y1="10" x2="35" y2="30" stroke="var(--accent)" strokeWidth="2" />
              <line x1="35" y1="35" x2="35" y2="45" stroke="var(--accent)" strokeWidth="2" />
              <line x1="35" y1="50" x2="35" y2="70" stroke="var(--accent)" strokeWidth="2" />
            </g>
          );
        case 'OpAmp':
          return (
            <g>
              <polygon points="0,-10 0,90 120,40" fill="var(--panel-bg)" stroke="var(--accent)" strokeWidth="2" />
              <text x="15" y="10" fill="var(--accent)" fontSize="14">+</text>
              <text x="15" y="70" fill="var(--accent)" fontSize="14">-</text>
            </g>
          );
        case 'Capacitor':
          return (
            <g>
              <line x1="0" y1="0" x2="35" y2="0" stroke="var(--accent)" strokeWidth="2" />
              <line x1="35" y1="-15" x2="35" y2="15" stroke="var(--accent)" strokeWidth="2" />
              <line x1="45" y1="-15" x2="45" y2="15" stroke="var(--accent)" strokeWidth="2" />
              <line x1="45" y1="0" x2="80" y2="0" stroke="var(--accent)" strokeWidth="2" />
            </g>
          );
        case 'ModulePort':
          return (
            <g>
              <circle cx="40" cy="0" r="10" fill="var(--panel-bg)" stroke="#a855f7" strokeWidth="2" />
              <circle cx="40" cy="0" r="4" fill="#a855f7" />
            </g>
          );
        default:
          return <rect width="40" height="40" fill="none" stroke="var(--accent)" strokeWidth="2" />;
      }
    };

    const handleWheel = (e: React.WheelEvent) => {
      // Prevent default scrolling handled by generic div wrapping, 
      // but in React onWheel passive is an issue, so we just adjust zoom.
      const zoomSensitivity = 0.002;
      setZoom(z => Math.max(0.1, Math.min(5, z - e.deltaY * zoomSensitivity)));
    };
    const handleMouseDown = (e: React.MouseEvent) => {
      setIsDragging(true);
      setDragStart({ x: e.clientX - pan.x, y: e.clientY - pan.y });
    };
    const handleMouseMove = (e: React.MouseEvent) => {
      if (isDragging) {
        setPan({ x: e.clientX - dragStart.x, y: e.clientY - dragStart.y });
      }
    };
    const handleMouseUp = () => setIsDragging(false);
    const handleMouseLeave = () => setIsDragging(false);

    return (
      <svg 
        width="100%" height="500" 
        style={{
          background: 'var(--bg-color)', 
          borderRadius: '8px', 
          border: '1px solid var(--border-color)',
          cursor: isDragging ? 'grabbing' : 'grab',
          userSelect: 'none'
        }}
        onWheel={handleWheel}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseLeave}
      >
        <g transform={`translate(${pan.x}, ${pan.y}) scale(${zoom})`}>
        
        {/* Çizgiler (Bağlantılar) */}
        {layout.wires.map((wire, i) => {
          const pts = wire.points
            .map(([x, y]) => `${x * SCALE + OFFSET_X},${y * SCALE + OFFSET_Y}`)
            .join(' ');
          return (
            <polyline 
              key={`wire-${i}`} 
              points={pts}
              fill="none"
              stroke="#38bdf8" 
              strokeWidth="2" 
              opacity="0.8"
            />
          );
        })}

        {/* Bileşenler */}
        {Object.entries(layout.components).map(([name, comp]) => {
          const cx = comp.x * SCALE + OFFSET_X;
          const cy = comp.y * SCALE + OFFSET_Y;
          const rot = (comp.rotation || 0) * 90;
          return (
          <g key={name} transform={`translate(${cx}, ${cy})`}>
            <g transform={`rotate(${rot})`}>
              {renderSymbol(comp.comp_type)}
            </g>
            
            {/* Etiketler (Her zaman düz durur) */}
            <text 
              x={
                comp.rotation === 1 ? (-comp.height * SCALE) / 2 :
                comp.rotation === 2 ? (-comp.width * SCALE) / 2 :
                comp.rotation === 3 ? (comp.height * SCALE) / 2 :
                (comp.width * SCALE) / 2
              } 
              y="-15" fill="#cccccc" fontSize="14" fontWeight="bold" textAnchor="middle">
              {name}
            </text>
          </g>
          );
        })}
        </g>
      </svg>
    );
  };

  return (
    <div className="app-container">
      <div className="panel left-panel">
        <div className="panel-header">
          <Code2 size={18} color="var(--accent)" />
          <span>NetLang Editor</span>
          <button className="compile-btn" onClick={compileCode} disabled={!isWasmLoaded}>
            <Play size={14} style={{ display: 'inline', marginRight: 5, verticalAlign: 'middle' }} /> 
            {isWasmLoaded ? 'Derle & ERC' : 'WASM...'}
          </button>
        </div>
        <div className="editor-container">
          <Editor
            height="100%"
            defaultLanguage="plaintext"
            theme="vs-dark"
            value={code}
            onChange={(value) => setCode(value || '')}
            options={{ minimap: { enabled: false }, fontSize: 15 }}
          />
        </div>
      </div>
      <div className="panel right-panel">
        <div className="panel-header" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
            <CircuitBoard size={18} color="var(--success)" />
            <span>Live Schematic Render</span>
          </div>
          <div style={{ display: 'flex', gap: '8px' }}>
            <button 
              className="compile-btn" 
              style={{ background: '#333', color: '#ccc', padding: '4px 10px', fontSize: '12px' }}
              onClick={() => {
                if(kicadSch) {
                  const blob = new Blob([kicadSch], { type: 'text/plain' });
                  const url = URL.createObjectURL(blob);
                  const a = document.createElement('a');
                  a.href = url;
                  a.download = 'circuit.kicad_sch';
                  a.click();
                }
              }}>
              <Download size={14} style={{ display: 'inline', marginRight: 5, verticalAlign: 'middle' }} /> 
              KiCad
            </button>
            <button 
              className="compile-btn" 
              style={{ background: '#333', color: '#ccc', padding: '4px 10px', fontSize: '12px' }}
              onClick={() => {
                if(spiceNetlist) {
                  const blob = new Blob([spiceNetlist], { type: 'text/plain' });
                  const url = URL.createObjectURL(blob);
                  const a = document.createElement('a');
                  a.href = url;
                  a.download = 'circuit.spice';
                  a.click();
                }
              }}>
              <Download size={14} style={{ display: 'inline', marginRight: 5, verticalAlign: 'middle' }} /> 
              SPICE
            </button>
          </div>
        </div>
        <div className="canvas-container" style={{ flex: 2 }}>
          {renderCircuit()}
        </div>
        
        <div className="panel-header" style={{ borderTop: '1px solid var(--border-color)', borderBottom: 'none' }}>
          <Code2 size={18} color="#f59e0b" />
          <span>Generated SPICE Netlist</span>
        </div>
        <div className="terminal-container" style={{ height: '120px', color: '#f59e0b', background: '#1e293b' }}>
          {spiceNetlist ? (
            <pre style={{ margin: 0 }}>{spiceNetlist}</pre>
          ) : (
            <div style={{ color: 'var(--text-muted)' }}>ERC hataları giderildiğinde SPICE netlist üretilecektir...</div>
          )}
        </div>

        <div className="panel-header" style={{ borderTop: '1px solid var(--border-color)', borderBottom: 'none' }}>
          <TerminalIcon size={18} color="var(--text-muted)" />
          <span>ERC Terminal (Self-Healing Log)</span>
        </div>
        <div className="terminal-content">
          {success && <div className="success-msg">ERC Başarılı: Şema güncellendi, 0 Hata. (Rust & WASM Engine)</div>}
          {errors.map((e, idx) => (
            <div key={idx} className={e.type === 'error' ? 'error-msg' : ''}>
              [{e.type ? e.type.toUpperCase() : 'ERROR'}] {e.message}
            </div>
          ))}
          {errors.length === 0 && !success && (
            <div style={{ color: 'var(--text-muted)' }}>Derlemeye hazır...</div>
          )}
        </div>
      </div>
    </div>
  );
}

export default App;
