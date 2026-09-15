import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  decodeWorkspaceDraft,
  documentNameFromFile,
  encodeWorkspaceDraft,
  MAX_DOCUMENT_SOURCE_BYTES,
  normalizeDocumentName,
  saveWithNativeFilePicker,
  sanitizeFileStem,
  type KessetsuFileHandle,
  WEB_DRAFT_SCHEMA_VERSION,
} from './document';

describe('Web document contract', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('normalizes display names and creates portable deterministic file stems', () => {
    expect(normalizeDocumentName('  Precision   Filter  ')).toBe('Precision Filter');
    expect(documentNameFromFile('Four Stage Amp.kess')).toBe('Four Stage Amp');
    expect(sanitizeFileStem('Çıkış / Filter: Rev A')).toBe('c-k-s-filter-rev-a');
    expect(sanitizeFileStem('***')).toBe('circuit');
    expect(() => normalizeDocumentName('   ')).toThrow(/Circuit name/);
  });

  it('round-trips only the current bounded draft schema', () => {
    const encoded = encodeWorkspaceDraft('RC Low-pass', 'net GND\n', true, new Date('2026-09-14T12:00:00Z'));
    expect(decodeWorkspaceDraft(encoded)).toEqual({
      schema_version: WEB_DRAFT_SCHEMA_VERSION,
      saved_at: '2026-09-14T12:00:00.000Z',
      name: 'RC Low-pass',
      source: 'net GND\n',
      dirty: true,
    });
    expect(decodeWorkspaceDraft('{"schema_version":"future"}')).toBeNull();
    expect(decodeWorkspaceDraft('{broken')).toBeNull();
    expect(() => encodeWorkspaceDraft(null, 'x'.repeat(MAX_DOCUMENT_SOURCE_BYTES + 1), true)).toThrow(/1 MiB/);
  });

  it('reuses a selected native handle for Save and repicks only for Save As', async () => {
    const writes: string[] = [];
    const suggestedNames: string[] = [];
    const handles = [1, 2].map((id): KessetsuFileHandle => ({
      name: `handle-${id}.kess`,
      getFile: async () => new File([], `handle-${id}.kess`),
      createWritable: async () => ({
        write: async (data: Blob) => { writes.push(`${id}:${await data.text()}`); },
        close: async () => undefined,
      }),
    }));
    let pickerCalls = 0;
    vi.stubGlobal('showSaveFilePicker', async (options: { suggestedName: string }) => {
      suggestedNames.push(options.suggestedName);
      return handles[pickerCalls++];
    });

    const first = await saveWithNativeFilePicker('first', 'precision-filter.kess', null, false);
    expect(first.status).toBe('saved');
    if (first.status !== 'saved') throw new Error('Expected native save');
    const second = await saveWithNativeFilePicker('second', 'precision-filter.kess', first.handle, false);
    const third = await saveWithNativeFilePicker('third', 'precision-filter.kess', first.handle, true);

    expect(second.status).toBe('saved');
    expect(third.status).toBe('saved');
    expect(pickerCalls).toBe(2);
    expect(suggestedNames).toEqual(['precision-filter.kess', 'precision-filter.kess']);
    expect(writes).toEqual(['1:first', '1:second', '2:third']);
  });

  it('distinguishes an unavailable picker and a cancelled Save As from a successful save', async () => {
    expect(await saveWithNativeFilePicker('source', 'circuit.kess', null, false)).toEqual({ status: 'unsupported' });
    vi.stubGlobal('showSaveFilePicker', async () => {
      throw new DOMException('Cancelled by user', 'AbortError');
    });
    await expect(saveWithNativeFilePicker('source', 'circuit.kess', null, true))
      .resolves.toEqual({ status: 'cancelled' });
  });
});
