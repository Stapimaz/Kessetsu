import { describe, expect, it } from 'vitest';
import type { ModelManifest } from './domain';
import {
  MAX_SHARE_SOURCE_BYTES,
  assertSharedPackages,
  decodeShareFragment,
  encodeShareFragment,
} from './share';

const compileSchema = 'kessetsu.compile.v4';
const manifest: ModelManifest = {
  schema_version: 'kessetsu.models.v2',
  models: [],
  packages: [{ name: 'kessetsu_analog', version: '1.0.0' }],
};

describe('versioned circuit share URLs', () => {
  it('round-trips UTF-8 source and exact package versions', async () => {
    const source = 'model_include kessetsu_analog 1.0.0\n// Ω ölçümü\n';
    const fragment = await encodeShareFragment(source, compileSchema, manifest, 'Precision filter');
    const decoded = await decodeShareFragment(fragment, compileSchema);
    expect(decoded).toMatchObject({ name: 'Precision filter', source, packages: manifest.packages });
    expect(() => assertSharedPackages(decoded!, manifest)).not.toThrow();
  });

  it('keeps unnamed version-one links compatible and validates optional names', async () => {
    const decoded = await decodeShareFragment(await encodeShareFragment('net GND\n', compileSchema, null), compileSchema);
    expect(decoded?.name).toBeUndefined();
    await expect(encodeShareFragment('net GND\n', compileSchema, null, '   ')).rejects.toThrow(/circuit name/);
  });

  it.each(['#kessetsu=2.abc', '#kessetsu=1.***', '#anything'])('rejects malformed or unsupported input: %s', async (fragment) => {
    await expect(decodeShareFragment(fragment, compileSchema)).rejects.toThrow();
  });

  it('rejects a small compressed decompression bomb before parsing', async () => {
    const oversized = 'R'.repeat(MAX_SHARE_SOURCE_BYTES + 40 * 1024);
    const compressed = new Uint8Array(await new Response(
      new Blob([oversized]).stream().pipeThrough(new CompressionStream('gzip')),
    ).arrayBuffer());
    let binary = '';
    for (const byte of compressed) binary += String.fromCharCode(byte);
    const fragment = `#kessetsu=1.${btoa(binary).replaceAll('+', '-').replaceAll('/', '_').replace(/=+$/, '')}`;
    await expect(decodeShareFragment(fragment, compileSchema)).rejects.toThrow(/decompressed size limit/);
  });

  it('accepts v4 source shares in v5 without accepting unknown or newer schemas', async () => {
    const fragment = await encodeShareFragment('net GND\n', 'kessetsu.compile.v4', manifest, 'Legacy circuit');
    const decoded = await decodeShareFragment(fragment, 'kessetsu.compile.v5');
    expect(decoded?.name).toBe('Legacy circuit');
    expect(() => assertSharedPackages(decoded!, null)).toThrow(/package versions/);
    for (const unsupported of ['kessetsu.compile.v3', 'kessetsu.compile.v6']) {
      const other = await encodeShareFragment('net GND\n', unsupported, null);
      await expect(decodeShareFragment(other, 'kessetsu.compile.v5')).rejects.toThrow(/unsupported Core schema/);
    }
    const future = await encodeShareFragment('net GND\n', 'kessetsu.compile.v5', null);
    await expect(decodeShareFragment(future, 'kessetsu.compile.v4')).rejects.toThrow(/unsupported Core schema/);
  });

  it('rejects source above the authoring limit and package-manifest mismatches', async () => {
    await expect(encodeShareFragment('x'.repeat(MAX_SHARE_SOURCE_BYTES + 1), compileSchema, null)).rejects.toThrow(/64 KiB/);
    const decoded = await decodeShareFragment(await encodeShareFragment('net GND\n', compileSchema, manifest), compileSchema);
    expect(() => assertSharedPackages(decoded!, null)).toThrow(/package versions/);
  });
});
