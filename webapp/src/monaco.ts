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
