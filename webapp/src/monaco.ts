import { loader } from '@monaco-editor/react';
import * as monaco from 'monaco-editor/editor/editor.api';
import EditorWorker from 'monaco-editor/editor/editor.worker?worker';
import { completionsForLine, hoverDocumentation } from './kessetsuLanguage';

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

monaco.languages.register({ id: 'kessetsu' });
monaco.editor.defineTheme('kessetsu-dark', {
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
monaco.languages.setMonarchTokensProvider('kessetsu', {
  tokenizer: {
    root: [
      [/\/\/.*$/, 'comment'],
      [/\b(param|net|connect|simulate|assert|model|model_include|subcircuit|external_subcircuit|module|use)\b/, 'keyword'],
      [/\b(source|current_source|resistor|capacitor|inductor|diode|transistor|mosfet|opamp)\b/, 'type'],
      [/\b(op|tran|ac|dc|dec|oct|lin|sine|sine_ac|pulse|pwl)\b/, 'keyword.control'],
      [/\b(value|min|max|peak|average|avg|rms|gain|gain_at|lower_cutoff|upper_cutoff|bandwidth|cutoff|frequency|phase|output_power|efficiency|thd|clipping|dissipation|rise_time|fall_time|settling_time|overshoot|energy)\b/, 'function'],
      [/-?(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?(?:T|G|meg|M|k|m|u|µ|n|p)?(?:V|A|Ohm|F|H|Hz|s|W|J|deg|%)?\b/i, 'number'],
      [/[A-Za-z_][A-Za-z0-9_]*/, 'identifier'],
      [/[(){},.:]/, 'delimiter'],
      [/(?:<=|>=|==|<|>|[=+*/-])/, 'operator'],
    ],
  },
});

monaco.languages.registerCompletionItemProvider('kessetsu', {
  provideCompletionItems(model, position) {
    const range = model.getWordUntilPosition(position);
    const lineBeforeCursor = model.getLineContent(position.lineNumber).slice(0, position.column - 1);
    return {
      suggestions: completionsForLine(lineBeforeCursor).map((item) => ({
        label: item.label,
        kind: monaco.languages.CompletionItemKind.Snippet,
        detail: item.detail,
        documentation: item.detail,
        insertText: item.insertText,
        insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
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

monaco.languages.registerHoverProvider('kessetsu', {
  provideHover(model, position) {
    const word = model.getWordAtPosition(position);
    const documentation = word && hoverDocumentation.get(word.word);
    if (!word || !documentation) return null;
    return {
      range: new monaco.Range(position.lineNumber, word.startColumn, position.lineNumber, word.endColumn),
      contents: [{ value: `**${word.word}**` }, { value: documentation }],
    };
  },
});

export { monaco };
