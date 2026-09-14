import { describe, expect, it } from 'vitest';
import {
  decodeWorkspaceDraft,
  documentNameFromFile,
  encodeWorkspaceDraft,
  MAX_DOCUMENT_SOURCE_BYTES,
  normalizeDocumentName,
  sanitizeFileStem,
  WEB_DRAFT_SCHEMA_VERSION,
} from './document';

describe('Web document contract', () => {
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
});
