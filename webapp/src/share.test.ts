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
    const fragment = await encodeShareFragment(source, compileSchema, manifest);
    const decoded = await decodeShareFragment(fragment, compileSchema);
    expect(decoded).toMatchObject({ source, packages: manifest.packages });
    expect(() => assertSharedPackages(decoded!, manifest)).not.toThrow();
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

  it('rejects source above the authoring limit and package-manifest mismatches', async () => {
    await expect(encodeShareFragment('x'.repeat(MAX_SHARE_SOURCE_BYTES + 1), compileSchema, null)).rejects.toThrow(/64 KiB/);
    const decoded = await decodeShareFragment(await encodeShareFragment('net GND\n', compileSchema, manifest), compileSchema);
    expect(() => assertSharedPackages(decoded!, null)).toThrow(/package versions/);
  });
});
