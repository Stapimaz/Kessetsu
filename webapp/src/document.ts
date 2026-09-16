export const WEB_DRAFT_SCHEMA_VERSION = 'kessetsu.web-draft.v1';
export const WEB_DRAFT_STORAGE_KEY = 'kessetsu.workspace.draft.v1';
export const PREVIOUS_CIRCUIT_STORAGE_KEY = 'kessetsu.workspace.previous.v1';
const TOOL_CIRCUIT_STORAGE_KEY = 'kessetsu.workspace.tool.v1';
export const MAX_DOCUMENT_NAME_LENGTH = 80;
export const MAX_DOCUMENT_SOURCE_BYTES = 1024 * 1024;

export interface WorkspaceDraft {
  schema_version: typeof WEB_DRAFT_SCHEMA_VERSION;
  saved_at: string;
  name: string | null;
  source: string;
  dirty: boolean;
}

export interface KessetsuFileHandle {
  readonly name: string;
  getFile(): Promise<File>;
  createWritable(): Promise<{
    write(data: Blob): Promise<void>;
    close(): Promise<void>;
  }>;
}

type FilePickerHost = typeof globalThis & {
  showOpenFilePicker?: (options: {
    multiple: false;
    types: Array<{ description: string; accept: Record<string, string[]> }>;
  }) => Promise<KessetsuFileHandle[]>;
  showSaveFilePicker?: (options: {
    suggestedName: string;
    types: Array<{ description: string; accept: Record<string, string[]> }>;
  }) => Promise<KessetsuFileHandle>;
};

export type NativeOpenResult =
  | { status: 'unsupported' }
  | { status: 'cancelled' }
  | { status: 'selected'; file: File; handle: KessetsuFileHandle };

export type NativeSaveResult =
  | { status: 'unsupported' }
  | { status: 'cancelled' }
  | { status: 'saved'; handle: KessetsuFileHandle };

const kessetsuFilePickerTypes = [{
  description: 'Kessetsu circuit',
  accept: { 'text/plain': ['.kess'] },
}];

function filePickerHost(): FilePickerHost {
  return globalThis as unknown as FilePickerHost;
}

export function nativeFileSavingSupported(): boolean {
  return typeof filePickerHost().showSaveFilePicker === 'function';
}

function isPickerCancellation(cause: unknown): boolean {
  return cause instanceof DOMException && cause.name === 'AbortError';
}

export async function openWithNativeFilePicker(): Promise<NativeOpenResult> {
  const picker = filePickerHost().showOpenFilePicker;
  if (!picker) return { status: 'unsupported' };
  try {
    const [handle] = await picker({ multiple: false, types: kessetsuFilePickerTypes });
    if (!handle) return { status: 'cancelled' };
    return { status: 'selected', handle, file: await handle.getFile() };
  } catch (cause: unknown) {
    if (isPickerCancellation(cause)) return { status: 'cancelled' };
    throw cause;
  }
}

export async function saveWithNativeFilePicker(
  source: string,
  suggestedName: string,
  currentHandle: KessetsuFileHandle | null,
  forceSaveAs: boolean,
): Promise<NativeSaveResult> {
  let handle = forceSaveAs ? null : currentHandle;
  if (!handle) {
    const picker = filePickerHost().showSaveFilePicker;
    if (!picker) return { status: 'unsupported' };
    try {
      handle = await picker({ suggestedName, types: kessetsuFilePickerTypes });
    } catch (cause: unknown) {
      if (isPickerCancellation(cause)) return { status: 'cancelled' };
      throw cause;
    }
  }

  try {
    const writable = await handle.createWritable();
    await writable.write(new Blob([source], { type: 'text/plain;charset=utf-8' }));
    await writable.close();
    return { status: 'saved', handle };
  } catch (cause: unknown) {
    if (isPickerCancellation(cause)) return { status: 'cancelled' };
    throw cause;
  }
}

function utf8Length(value: string): number {
  return new TextEncoder().encode(value).byteLength;
}

export function normalizeDocumentName(value: string): string {
  const name = value.trim().replace(/\s+/g, ' ');
  const hasControlCharacter = [...name].some((character) => {
    const codePoint = character.codePointAt(0) ?? 0;
    return codePoint <= 31 || codePoint === 127;
  });
  if (!name || name.length > MAX_DOCUMENT_NAME_LENGTH || hasControlCharacter) {
    throw new Error(`Circuit name must be 1-${MAX_DOCUMENT_NAME_LENGTH} printable characters`);
  }
  return name;
}

export function documentNameFromFile(fileName: string): string {
  const withoutExtension = fileName.replace(/\.kess$/i, '');
  return normalizeDocumentName(withoutExtension || 'Untitled circuit');
}

export function sanitizeFileStem(name: string): string {
  const normalized = name
    .normalize('NFKD')
    .replace(/[\u0300-\u036f]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 64)
    .replace(/-+$/g, '');
  return normalized || 'circuit';
}

export function encodeWorkspaceDraft(name: string | null, source: string, dirty: boolean, savedAt = new Date()): string {
  if (utf8Length(source) > MAX_DOCUMENT_SOURCE_BYTES) {
    throw new Error('Circuit source exceeds the 1 MiB browser draft limit');
  }
  const draft: WorkspaceDraft = {
    schema_version: WEB_DRAFT_SCHEMA_VERSION,
    saved_at: savedAt.toISOString(),
    name: name === null ? null : normalizeDocumentName(name),
    source,
    dirty,
  };
  return JSON.stringify(draft);
}

export function writeWorkspaceDraft(
  storage: Pick<Storage, 'setItem'>,
  name: string | null,
  source: string,
  dirty: boolean,
): void {
  storage.setItem(WEB_DRAFT_STORAGE_KEY, encodeWorkspaceDraft(name, source, dirty));
}

export function decodeWorkspaceDraft(raw: string | null): WorkspaceDraft | null {
  if (!raw) return null;
  try {
    const value = JSON.parse(raw) as Partial<WorkspaceDraft>;
    if (value.schema_version !== WEB_DRAFT_SCHEMA_VERSION || typeof value.source !== 'string') return null;
    if (utf8Length(value.source) > MAX_DOCUMENT_SOURCE_BYTES) return null;
    if (typeof value.saved_at !== 'string' || !Number.isFinite(Date.parse(value.saved_at))) return null;
    const name = value.name === null || value.name === undefined ? null : normalizeDocumentName(value.name);
    if (typeof value.dirty !== 'boolean') return null;
    return {
      schema_version: WEB_DRAFT_SCHEMA_VERSION,
      saved_at: value.saved_at,
      name,
      source: value.source,
      dirty: value.dirty,
    };
  } catch {
    return null;
  }
}

export function downloadTextFile(source: string, fileName: string): void {
  const blob = new Blob([source], { type: 'text/plain;charset=utf-8' });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = fileName;
  anchor.click();
  URL.revokeObjectURL(url);
}

/** Stage a tool circuit while retaining the workspace it will replace. */
export function stageToolCircuit(name: string, source: string): void {
  const previous = decodeWorkspaceDraft(globalThis.localStorage.getItem(WEB_DRAFT_STORAGE_KEY));
  if (previous && previous.source !== source) {
    globalThis.localStorage.setItem(PREVIOUS_CIRCUIT_STORAGE_KEY, JSON.stringify(previous));
  }
  globalThis.sessionStorage.setItem(TOOL_CIRCUIT_STORAGE_KEY, encodeWorkspaceDraft(name, source, true));
}

export function takeToolCircuit(): WorkspaceDraft | null {
  const staged = decodeWorkspaceDraft(globalThis.sessionStorage.getItem(TOOL_CIRCUIT_STORAGE_KEY));
  globalThis.sessionStorage.removeItem(TOOL_CIRCUIT_STORAGE_KEY);
  return staged;
}
