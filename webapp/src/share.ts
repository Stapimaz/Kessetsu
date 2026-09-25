import type { ModelManifest } from './domain';

export const SHARE_SCHEMA_VERSION = 'kessetsu.share.v1';
export const MAX_SHARE_SOURCE_BYTES = 64 * 1024;
export const MAX_SHARE_COMPRESSED_BYTES = 64 * 1024;
export const MAX_SHARE_NAME_LENGTH = 80;
const MAX_SHARE_ENVELOPE_BYTES = 96 * 1024;
const PREFIX = '#kessetsu=1.';

export interface SharePackage {
  name: string;
  version: string;
}

export interface ShareEnvelope {
  schema_version: typeof SHARE_SCHEMA_VERSION;
  compile_schema_version: string;
  name?: string;
  source: string;
  packages: SharePackage[];
}

function normalizeName(value: unknown): string | undefined {
  if (value === undefined) return undefined;
  if (typeof value !== 'string') throw new Error('Share payload circuit name must be text');
  const name = value.trim();
  const hasControlCharacter = Array.from(name).some((character) => {
    const codePoint = character.codePointAt(0) ?? 0;
    return codePoint < 32 || codePoint === 127;
  });
  if (!name || Array.from(name).length > MAX_SHARE_NAME_LENGTH || hasControlCharacter) {
    throw new Error(`Share payload circuit name must be 1-${MAX_SHARE_NAME_LENGTH} printable characters`);
  }
  return name;
}

function utf8Length(value: string): number {
  return new TextEncoder().encode(value).byteLength;
}

function base64UrlEncode(bytes: Uint8Array): string {
  let binary = '';
  for (let offset = 0; offset < bytes.length; offset += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(offset, offset + 0x8000));
  }
  return btoa(binary).replaceAll('+', '-').replaceAll('/', '_').replace(/=+$/, '');
}

function base64UrlDecode(value: string): Uint8Array {
  if (!/^[A-Za-z0-9_-]+$/.test(value)) throw new Error('Share payload is not valid base64url');
  const padding = '='.repeat((4 - value.length % 4) % 4);
  const binary = atob(value.replaceAll('-', '+').replaceAll('_', '/') + padding);
  return Uint8Array.from(binary, (character) => character.charCodeAt(0));
}

async function transformWithLimit(
  bytes: Uint8Array,
  stream: CompressionStream | DecompressionStream,
  limit: number,
): Promise<Uint8Array> {
  const writer = stream.writable.getWriter();
  const writing = (async () => {
    const transferable = Uint8Array.from(bytes).buffer as ArrayBuffer;
    await writer.write(transferable);
    await writer.close();
  })();
  const reader = stream.readable.getReader();
  const chunks: Uint8Array[] = [];
  let length = 0;
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    length += value.byteLength;
    if (length > limit) {
      await reader.cancel().catch(() => undefined);
      await writing.catch(() => undefined);
      throw new Error('Share payload exceeds the decompressed size limit');
    }
    chunks.push(value);
  }
  const output = new Uint8Array(length);
  let offset = 0;
  for (const chunk of chunks) {
    output.set(chunk, offset);
    offset += chunk.byteLength;
  }
  await writing;
  return output;
}

function normalizePackages(packages: SharePackage[]): SharePackage[] {
  const normalized = packages.map(({ name, version }) => ({ name, version }))
    .sort((left, right) => `${left.name}@${left.version}`.localeCompare(`${right.name}@${right.version}`));
  const seen = new Set<string>();
  for (const entry of normalized) {
    if (!/^[A-Za-z0-9_.-]+$/.test(entry.name) || !/^\d+\.\d+\.\d+(?:[-+][A-Za-z0-9.-]+)?$/.test(entry.version)) {
      throw new Error('Share payload contains an invalid package identity');
    }
    if (seen.has(entry.name)) throw new Error('Share payload contains duplicate packages');
    seen.add(entry.name);
  }
  return normalized;
}

export async function encodeShareFragment(
  source: string,
  compileSchemaVersion: string,
  manifest: ModelManifest | null,
  name?: string,
): Promise<string> {
  if (utf8Length(source) > MAX_SHARE_SOURCE_BYTES) throw new Error('Circuit source exceeds the 64 KiB share limit');
  const envelope: ShareEnvelope = {
    schema_version: SHARE_SCHEMA_VERSION,
    compile_schema_version: compileSchemaVersion,
    ...(name === undefined ? {} : { name: normalizeName(name) }),
    source,
    packages: normalizePackages(manifest?.packages ?? []),
  };
  const compressed = await transformWithLimit(
    new TextEncoder().encode(JSON.stringify(envelope)),
    new CompressionStream('gzip'),
    MAX_SHARE_COMPRESSED_BYTES,
  );
  return `${PREFIX}${base64UrlEncode(compressed)}`;
}

function migrateShareEnvelope(raw: unknown): ShareEnvelope {
  if (!raw || typeof raw !== 'object') throw new Error('Share payload must be an object');
  const value = raw as Partial<ShareEnvelope>;
  switch (value.schema_version) {
    case SHARE_SCHEMA_VERSION:
      return value as ShareEnvelope;
    default:
      throw new Error('Unsupported share schema version');
  }
}

export async function decodeShareFragment(fragment: string, expectedCompileSchema: string): Promise<ShareEnvelope | null> {
  if (!fragment) return null;
  if (!fragment.startsWith(PREFIX)) throw new Error('Unsupported Kessetsu share URL version');
  const encoded = fragment.slice(PREFIX.length);
  if (encoded.length > Math.ceil(MAX_SHARE_COMPRESSED_BYTES * 4 / 3)) {
    throw new Error('Share payload exceeds the compressed size limit');
  }
  const compressed = base64UrlDecode(encoded);
  if (compressed.byteLength > MAX_SHARE_COMPRESSED_BYTES) throw new Error('Share payload exceeds the compressed size limit');
  const decoded = await transformWithLimit(
    compressed,
    new DecompressionStream('gzip'),
    MAX_SHARE_ENVELOPE_BYTES,
  );
  let raw: unknown;
  try {
    raw = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(decoded));
  } catch {
    throw new Error('Share payload is not valid UTF-8 JSON');
  }
  const value = migrateShareEnvelope(raw);
  // These report-schema revisions preserve the same source semantics. Shared source is always
  // recompiled by the current Core and exact model-package versions are still checked separately.
  // Keep the matrix explicit so a future compiler revision cannot inherit compatibility by accident.
  const compatibleLegacy = ({
    'kessetsu.compile.v6': ['kessetsu.compile.v4', 'kessetsu.compile.v5'],
    'kessetsu.compile.v7': ['kessetsu.compile.v4', 'kessetsu.compile.v5', 'kessetsu.compile.v6'],
    'kessetsu.compile.v8': ['kessetsu.compile.v4', 'kessetsu.compile.v5', 'kessetsu.compile.v6', 'kessetsu.compile.v7'],
  } as Record<string, string[]>)[expectedCompileSchema]?.includes(value.compile_schema_version) ?? false;
  if (value.compile_schema_version !== expectedCompileSchema && !compatibleLegacy) {
    throw new Error('Shared circuit requires an unsupported Core schema');
  }
  if (typeof value.source !== 'string' || utf8Length(value.source) > MAX_SHARE_SOURCE_BYTES) {
    throw new Error('Shared circuit source is missing or exceeds 64 KiB');
  }
  if (!Array.isArray(value.packages)) throw new Error('Share payload package manifest is missing');
  return { ...value, name: normalizeName(value.name), packages: normalizePackages(value.packages) } as ShareEnvelope;
}

export function assertSharedPackages(envelope: ShareEnvelope, manifest: ModelManifest | null): void {
  const actual = normalizePackages(manifest?.packages ?? []);
  if (JSON.stringify(actual) !== JSON.stringify(envelope.packages)) {
    throw new Error('Compiled package versions do not match the shared URL manifest');
  }
}
