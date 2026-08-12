import { loader } from '@monaco-editor/react';
import * as monaco from 'monaco-editor/editor/editor.api';
import EditorWorker from 'monaco-editor/editor/editor.worker?worker';

type MonacoEnvironment = typeof globalThis & {
  MonacoEnvironment: {
    getWorker(): Worker;
  };
};

(globalThis as MonacoEnvironment).MonacoEnvironment = {
  getWorker() {
    return new EditorWorker();
  },
};

loader.config({ monaco });

monaco.languages.register({ id: 'netlang' });
monaco.editor.defineTheme('netlang-dark', {
  base: 'vs-dark',
  inherit: true,
  rules: [
    { token: 'keyword', foreground: '7DD3FC' },
    { token: 'keyword.control', foreground: 'C4B5FD' },
    { token: 'type', foreground: '67E8F9' },
    { token: 'function', foreground: 'FDE68A' },
    { token: 'number', foreground: 'A7F3D0' },
    { token: 'comment', foreground: '64748B', fontStyle: 'italic' },
  ],
  colors: { 'editor.background': '#111722', 'editorLineNumber.foreground': '#526078' },
});
monaco.languages.setMonarchTokensProvider('netlang', {
  tokenizer: {
    root: [
      [/\/\/.*$/, 'comment'],
      [/\b(net|connect|simulate|assert|model|model_include|subcircuit|module|use)\b/, 'keyword'],
      [/\b(source|current_source|resistor|capacitor|inductor|diode|transistor|mosfet|opamp)\b/, 'type'],
      [/\b(op|tran|ac|dc|dec|oct|lin)\b/, 'keyword.control'],
      [/\b(value|min|max|peak|average|avg|rms|gain|bandwidth|cutoff|frequency|phase|output_power|efficiency|thd|clipping|dissipation)\b/, 'function'],
      [/-?(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?(?:T|G|meg|M|k|m|u|µ|n|p)?(?:V|A|Ohm|F|H|Hz|s|W|deg|%)?\b/i, 'number'],
      [/[A-Za-z_][A-Za-z0-9_]*/, 'identifier'],
      [/[(),.]/, 'delimiter'],
      [/(?:<=|>=|==|<|>)/, 'operator'],
    ],
  },
});

const declarations = [
  ['source', 'Independent voltage source: source V1 5V'],
  ['current_source', 'Independent current source: current_source I1 10mA'],
  ['resistor', 'Two-terminal resistor: resistor R1 1k'],
  ['capacitor', 'Two-terminal capacitor: capacitor C1 100nF'],
  ['inductor', 'Two-terminal inductor: inductor L1 10mH'],
  ['diode', 'Diode with optional model: diode D1 1N4148'],
  ['transistor', 'BJT: transistor Q1 npn 2N3904'],
  ['mosfet', 'MOSFET: mosfet M1 nmos IRF540'],
  ['opamp', 'Five-pin op-amp: opamp U1 NLANG_OPAMP_V1'],
  ['connect', 'Connect canonical pins or a named net'],
  ['simulate', 'Add typed op, tran, ac or dc analysis'],
  ['assert', 'Add an executable engineering requirement'],
] as const;

monaco.languages.registerCompletionItemProvider('netlang', {
  provideCompletionItems(model, position) {
    const range = model.getWordUntilPosition(position);
    return {
      suggestions: declarations.map(([label, documentation]) => ({
        label,
        kind: monaco.languages.CompletionItemKind.Keyword,
        documentation,
        insertText: label,
        range: {
          startLineNumber: position.lineNumber,
          endLineNumber: position.lineNumber,
          startColumn: range.startColumn,
          endColumn: range.endColumn,
        },
      })),
    };
  },
});

monaco.languages.registerHoverProvider('netlang', {
  provideHover(model, position) {
    const word = model.getWordAtPosition(position);
    const declaration = word && declarations.find(([label]) => label === word.word);
    if (!word || !declaration) return null;
    return {
      range: new monaco.Range(position.lineNumber, word.startColumn, position.lineNumber, word.endColumn),
      contents: [{ value: `**${declaration[0]}**` }, { value: declaration[1] }],
    };
  },
});

export { monaco };
