import React, { useState, useEffect } from 'react';
import Editor from '@monaco-editor/react';
import { Play, Code2, CircuitBoard, Terminal as TerminalIcon } from 'lucide-react';
import init, { compile_netlang } from 'netlang-core';

const DEFAULT_CODE = `// NetLang Micro-DSL MVP (Rust/WASM Core)
resistor R1 10k
battery B1 9V
capacitor C1 10uF

// Bilerek hatalı bağlantı yapmayı deneyin (Örn: R2.p1 yazın)
connect B1.plus R1.p1
connect R1.p2 C1.p1
connect C1.p2 B1.minus
`;

function App() {
  const [code, setCode] = useState(DEFAULT_CODE);
  const [program, setProgram] = useState<any>(null);
  const [errors, setErrors] = useState<any[]>([]);
  const [success, setSuccess] = useState<boolean>(false);
  const [isWasmLoaded, setIsWasmLoaded] = useState(false);

  useEffect(() => {
    init().then(() => {
      setIsWasmLoaded(true);
    });
  }, []);

  const compileCode = () => {
    if (!isWasmLoaded) return;
    
    try {
      const result = compile_netlang(code);
      
      if (result.parse_error) {
        setErrors([{ message: `Syntax Error: ${result.parse_error}`, type: 'error' }]);
        setSuccess(false);
      } else {
        const drcErrors = result.drc_errors || [];
        setProgram(result.ast);
        
        if (drcErrors.length > 0) {
          setErrors(drcErrors.map((e: any) => ({ message: e.message, type: 'error' })));
          setSuccess(false);
        } else {
          setErrors([]);
          setSuccess(true);
        }
      }
    } catch (e: any) {
      setErrors([{ message: `WASM Execution Error: ${e.message}`, type: 'error' }]);
      setSuccess(false);
    }
  };

  useEffect(() => {
    compileCode();
  }, [code, isWasmLoaded]);

  // MVP Simple Renderer inside component for simplicity
  const renderCircuit = () => {
    if (!program || !program.statements) return <div style={{color: '#94a3b8'}}>Henüz devreniz yok... Kod yazmaya başlayın.</div>;
    
    let x = 50;
    const components = program.statements
      .filter((s: any) => s.Decl)
      .map((s: any) => {
        const decl = s.Decl;
        const comp = {
          id: decl.name,
          type: decl.comp_type,
          value: decl.value,
          x: x,
          y: 100,
        };
        x += 150; 
        return comp;
      });

    const getCompPos = (name: string) => components.find((c: any) => c.id === name);

    const connections = program.statements
      .filter((s: any) => s.Connect)
      .map((s: any, index: number) => {
        const conn = s.Connect;
        const c1 = getCompPos(conn.pin1.component);
        const c2 = getCompPos(conn.pin2.component);
        if (!c1 || !c2) return null;

        return (
          <line
            key={`net-${index}`}
            x1={c1.x + 25}
            y1={c1.y + 25}
            x2={c2.x + 25}
            y2={c2.y + 25}
            stroke="#4ade80"
            strokeWidth="3"
          />
        );
      });

    return (
      <svg width="100%" height="100%">
        {connections}
        {components.map((c: any) => (
          <g key={c.id} transform={`translate(${c.x}, ${c.y})`}>
            <rect width="50" height="50" fill="#1e293b" stroke="#38bdf8" strokeWidth="2" rx="5" />
            <text x="25" y="20" fill="white" fontSize="12" textAnchor="middle">{c.id}</text>
            <text x="25" y="35" fill="#94a3b8" fontSize="10" textAnchor="middle">{c.value}</text>
            {c.type === 'Resistor' && <text x="25" y="-5" fill="#38bdf8" fontSize="16" textAnchor="middle">〰</text>}
            {c.type === 'Battery' && <text x="25" y="-5" fill="#f87171" fontSize="16" textAnchor="middle">🔋</text>}
          </g>
        ))}
      </svg>
    );
  };

  return (
    <div className="app-container">
      <div className="panel left-panel">
        <div className="panel-header">
          <Code2 size={18} color="var(--accent)" />
          <span>NetLang Editor (Rust Core)</span>
          <button className="compile-btn" onClick={compileCode} disabled={!isWasmLoaded}>
            <Play size={14} style={{ display: 'inline', marginRight: 5, verticalAlign: 'middle' }} /> 
            {isWasmLoaded ? 'Derle & DRC' : 'WASM Yükleniyor...'}
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
        <div className="panel-header">
          <CircuitBoard size={18} color="var(--success)" />
          <span>Live Schematic Render (WASM)</span>
        </div>
        <div className="canvas-container">
          {renderCircuit()}
        </div>
        <div className="panel-header" style={{ borderTop: '1px solid var(--border-color)', borderBottom: 'none' }}>
          <TerminalIcon size={18} color="var(--text-muted)" />
          <span>DRC Terminal (Self-Healing Log)</span>
        </div>
        <div className="terminal-container">
          {success && <div className="success-msg">DRC Başarılı: Şema güncellendi, 0 Hata. (Rust & WASM Engine)</div>}
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
