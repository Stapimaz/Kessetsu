export const WEB_DRAFT_SCHEMA_VERSION = 'kessetsu.web-draft.v1';
export const WEB_DRAFT_STORAGE_KEY = 'kessetsu.workspace.draft.v1';
export const MAX_DOCUMENT_NAME_LENGTH = 80;
export const MAX_DOCUMENT_SOURCE_BYTES = 1024 * 1024;

export interface WorkspaceDraft {
  schema_version: typeof WEB_DRAFT_SCHEMA_VERSION;
  saved_at: string;
  name: string | null;
  source: string;
  dirty: boolean;
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
