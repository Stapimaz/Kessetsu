// Build-time only: reuse actual React surfaces and reviewed Markdown sources.
// The editor stays lazy/client-only; documentation ships no JavaScript runtime.
import { build } from 'esbuild';
import { Marked, Renderer } from 'marked';
import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, posix, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const web = resolve(import.meta.dirname, '..');
const repo = resolve(web, '..');
const dist = resolve(web, 'dist');
const base = process.env.KESSETSU_BASE_PATH ?? '/';
const origin = 'https://kessetsu.com/';
const escape = (text) => String(text).replace(/[&<>"']/g, (char) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[char]);
const rendererPath = resolve(web, 'node_modules/.cache/kessetsu-public-renderer.mjs');
const bundled = await build({
  absWorkingDir: web,
  stdin: { contents: `
    import { createElement } from 'react';
    import { renderToStaticMarkup } from 'react-dom/server';
    import { LandingPage } from './src/components/LandingPage';
    import { InstallPage } from './src/components/InstallPage';
    import { BrandWordmark } from './src/components/BrandWordmark';
    import { CircuitToolsIndex, CircuitToolsPage } from './src/components/CircuitToolsPage';
    export const landing = () => renderToStaticMarkup(createElement(LandingPage));
    export const install = () => renderToStaticMarkup(createElement(InstallPage));
    export const wordmark = () => renderToStaticMarkup(createElement(BrandWordmark));
    export const tools = () => renderToStaticMarkup(createElement(CircuitToolsIndex));
    export const tool = (toolId) => renderToStaticMarkup(createElement(CircuitToolsPage, { toolId }));
  `, resolveDir: web, sourcefile: 'public-renderer.ts' },
  bundle: true, platform: 'node', format: 'esm', packages: 'external', write: false,
  outfile: rendererPath, jsx: 'automatic', loader: { '.css': 'empty' },
  define: { 'import.meta.env.BASE_URL': JSON.stringify(base), 'process.env.NODE_ENV': '"production"' },
  plugins: [{ name: 'source-text', setup(builder) {
    builder.onResolve({ filter: /\?raw$/ }, ({ path, resolveDir }) => ({ path: resolve(resolveDir, path.slice(0, -4)), namespace: 'source-text' }));
    builder.onLoad({ filter: /.*/, namespace: 'source-text' }, ({ path }) => ({ contents: readFileSync(path, 'utf8'), loader: 'text' }));
  } }],
});
mkdirSync(dirname(rendererPath), { recursive: true });
writeFileSync(rendererPath, bundled.outputFiles[0].contents);
const surfaces = await import(pathToFileURL(rendererPath).href);
const template = readFileSync(resolve(dist, 'index.html'), 'utf8');

function page(title, description, route, body, { scripts = true, extraCss = '' } = {}) {
  let html = template;
  const replace = (pattern, replacement) => {
    if (!pattern.test(html)) throw new Error(`Missing public-page template field: ${pattern}`);
    html = html.replace(pattern, () => replacement);
  };
  replace(/<title>.*?<\/title>/, `<title>${escape(title)}</title>`);
  for (const [attribute, key, value] of [
    ['name', 'description', description], ['property', 'og:title', title],
    ['property', 'og:description', description], ['property', 'og:url', `${origin}${route}`],
    ['name', 'twitter:title', title], ['name', 'twitter:description', description],
  ]) replace(new RegExp(`<meta ${attribute}="${key}" content="[^"]*"\\s*/?>`), `<meta ${attribute}="${key}" content="${escape(value)}" />`);
  replace(/<link rel="canonical" href="[^"]*"\s*\/?>/, `<link rel="canonical" href="${origin}${route}" />`);
  replace(/<div id="root"><\/div>/, `<div id="root">${body}</div>`);
  if (!scripts) html = html.replace(/<script\b[^>]*>[\s\S]*?<\/script>/g, '').replace(/<link[^>]*rel="modulepreload"[^>]*>/g, '');
  return html.replace('</head>', `${extraCss}</head>`);
}

writeFileSync(resolve(dist, 'index.html'), page(
  'Kessetsu — Circuit simulation, verification and schematics',
  'Free circuit simulation and automatic schematics in your browser, plus an AI-agent CLI. Verify SPICE requirements and export to KiCad, LTspice, SVG, PNG or PDF.',
  '', `${surfaces.landing()}<noscript><p class="public-noscript">The interactive Web Hub requires JavaScript. You can still read the <a href="${base}docs/">documentation</a> or <a href="${base}install/">install the local CLI</a>.</p></noscript>`,
));
const installCss = readdirSync(resolve(dist, 'assets')).find((file) => /^InstallPage-.*\.css$/.test(file));
if (!installCss) throw new Error('Installation stylesheet is missing.');
mkdirSync(resolve(dist, 'install'), { recursive: true });
writeFileSync(resolve(dist, 'install/index.html'), page(
  'Install Kessetsu CLI — Kessetsu',
  'Install the free kess CLI on Windows, macOS or Linux with one command. Verified downloads, no administrator access, updates and your first SPICE circuit.',
  'install/', `${surfaces.install()}<noscript><section class="public-noscript"><h2>Without JavaScript</h2><p>The steps above show Windows. On macOS or Linux, open Terminal and run <code>curl -fsSL https://kessetsu.com/install.sh | sh</code>. Then open a new terminal and run <code>kess --version</code>. Linux/macOS simulation also needs a system Ngspice installation as explained above.</p></section></noscript>`,
  { extraCss: `<link rel="stylesheet" href="${base}assets/${installCss}" />` },
));

const reviewed = JSON.parse(readFileSync(resolve(repo, 'docs/public-documents.json'), 'utf8')).files;
const toolsCss = readdirSync(resolve(dist, 'assets')).find((file) => /^CircuitToolsPage-.*\.css$/.test(file));
if (!toolsCss) throw new Error('Circuit tools stylesheet is missing.');
const toolPages = [
  ['tools/', 'Circuit tools — Kessetsu', 'Free circuit design tools: calculate a loaded voltage divider or RC filter, then open an editable circuit for simulation and schematic export.', surfaces.tools()],
  ['tools/voltage-divider/', 'Loaded voltage divider — Kessetsu', 'Calculate a loaded voltage divider with standard resistor values, output error, currents and power. Open the generated circuit for SPICE simulation.', surfaces.tool('divider')],
  ['tools/rc-lowpass/', 'RC low-pass filter — Kessetsu', 'Calculate RC low-pass filter values and achieved cutoff frequency. Choose E12/E24 components and open an editable circuit for AC simulation.', surfaces.tool('rc_lowpass')],
];
for (const [route, title, description, body] of toolPages) {
  mkdirSync(resolve(dist, route), { recursive: true });
  writeFileSync(resolve(dist, route, 'index.html'), page(title, description, route,
    `${body}<noscript><p class="public-noscript">Interactive calculations need JavaScript. The equations and assumptions above remain available; you can also use the local CLI.</p></noscript>`,
    { extraCss: `<link rel="stylesheet" href="${base}assets/${toolsCss}" />` }));
}
const documents = [...reviewed.filter((file) => file === 'docs/README.md' || /^docs\/(guides|reference)\/[^/]+\.md$/.test(file)), 'CHANGELOG.md'];
const routes = new Map(documents.map((file) => [file, file === 'docs/README.md' ? 'docs/' : file === 'CHANGELOG.md' ? 'changelog/' : file.replace(/\.md$/, '/')]));
// Public example sources and their open model sidecars are downloadable on-site.
for (const file of readdirSync(resolve(repo, 'examples'), { recursive: true })) {
  const relative = file.replaceAll('\\', '/');
  if (!/(?:\.(?:kess|kessreq|lib|md|csv|ipynb)|\.kess(?:study|import|compare|sim)\.json)$/.test(relative)) continue;
  const destination = resolve(dist, 'examples', relative);
  mkdirSync(dirname(destination), { recursive: true });
  copyFileSync(resolve(repo, 'examples', relative), destination);
}
const navigationGroups = [
  ['Getting started', ['docs/README.md', 'docs/guides/tutorial.md', 'docs/guides/web-editor.md', 'docs/guides/why-kessetsu.md', 'docs/guides/cookbook.md', 'docs/guides/parameter-studies.md', 'docs/guides/research-data.md', 'docs/guides/model-fitting.md', 'docs/guides/memristor-protocol.md', 'docs/guides/portable-research-packages.md', 'docs/guides/python-notebooks.md']],
  ['Reference', documents.filter((file) => file.startsWith('docs/reference/'))],
  ['Help and updates', ['docs/guides/troubleshooting.md', 'CHANGELOG.md']],
];
const shortTitles = { 'docs/README.md': 'Overview', 'docs/reference/supported-domain.md': 'Components and limits', 'docs/reference/model-catalog.md': 'Device models', 'docs/reference/cli.md': 'CLI', 'docs/reference/language.md': 'Language', 'docs/reference/exports.md': 'Export formats', 'docs/reference/measurements.md': 'Measurements', 'docs/reference/simulation-and-assertions.md': 'Simulation and assertions' };
const titleFor = (file) => readFileSync(resolve(repo, file), 'utf8').match(/^# (.+)$/m)?.[1].trim();
function linkFor(source, href) {
  if (/^(?:https?:|mailto:|#)/i.test(href)) return href;
  if (/^[a-z][a-z\d+.-]*:/i.test(href) || href.startsWith('//')) return '#';
  const [path, fragment] = href.split('#', 2);
  const target = posix.normalize(posix.join(posix.dirname(source), path));
  if (target === 'LICENSE') return `${base}LICENSE.txt`;
  if (target.startsWith('examples/') && existsExample(target)) return `${base}${target}` + (fragment ? `#${fragment}` : '');
  return (routes.has(target) ? `${base}${routes.get(target)}` : `https://github.com/Stapimaz/Kessetsu/blob/main/${target.split('/').map(encodeURIComponent).join('/')}`) + (fragment ? `#${fragment}` : '');
}
function existsExample(target) { return existsSync(resolve(dist, target)); }
for (const source of documents) {
  const markdown = readFileSync(resolve(repo, source), 'utf8');
  const title = markdown.match(/^# (.+)$/m)?.[1].trim();
  if (!title) throw new Error(`Public guide has no title: ${source}`);
  const renderer = new Renderer();
  const slugs = new Map();
  const sections = [];
  renderer.heading = function ({ tokens, depth, text }) {
    const slug = text.toLowerCase().replace(/<[^>]*>/g, '').replace(/[^\p{L}\p{N}\s_-]/gu, '').trim().replace(/\s/g, '-');
    const count = slugs.get(slug) ?? 0;
    slugs.set(slug, count + 1);
    const id = slug + (count ? `-${count}` : '');
    if (depth === 2) sections.push([id, text]);
    return `<h${depth} id="${escape(id)}">${this.parser.parseInline(tokens)}</h${depth}>\n`;
  };
  // Inputs are reviewed repository documents, never circuit/user input. Escape
  // raw HTML anyway; do not enable scripts or unsafe link schemes in published docs.
  renderer.html = ({ text }) => escape(text);
  renderer.link = function ({ href, title, tokens }) {
    return `<a href="${escape(linkFor(source, href))}"${title ? ` title="${escape(title)}"` : ''}>${this.parser.parseInline(tokens)}</a>`;
  };
  const content = new Marked({ renderer, async: false }).parse(markdown);
  const route = routes.get(source);
  const navigation = navigationGroups.map(([group, files]) => `<section><h2>${escape(group)}</h2>${files.map((file) => `<a href="${base}${routes.get(file)}"${file === source ? ' aria-current="page"' : ''}>${escape(shortTitles[file] ?? titleFor(file)?.replace(/^Tutorial: .*/, 'Tutorial').replace(/ Guide$/, ''))}</a>`).join('')}</section>`).join('');
  const toc = sections.length ? `<details class="docs-toc" open><summary>On this page</summary><nav aria-label="On this page">${sections.map(([id, text]) => `<a href="#${escape(id)}">${escape(text)}</a>`).join('')}</nav></details>` : '';
  const article = content.replace(/(<h1\b[^>]*>[\s\S]*?<\/h1>)/, `$1${toc}`);
  const body = `<main class="landing-page docs-page"><header class="landing-header"><a class="landing-wordmark" href="${base}" aria-label="Kessetsu home">${surfaces.wordmark()}</a><nav class="landing-nav" aria-label="Documentation navigation"><a href="${base}docs/">Docs</a><a href="${base}changelog/">Changelog</a><a href="${base}install/">Install CLI</a><a href="${base}#editor">Web Hub</a></nav></header><a class="docs-skip" href="#doc-content">Skip to content</a><div class="docs-layout"><aside class="docs-sidebar"><details open><summary>Documentation</summary><nav aria-label="Documentation topics">${navigation}</nav></details></aside><article class="docs-article" id="doc-content">${article}</article></div><footer class="landing-footer"><a href="${base}docs/">Documentation</a><a href="${base}changelog/">Changelog</a><a href="https://github.com/Stapimaz/Kessetsu/blob/main/${source}">Edit this page on GitHub</a></footer></main>`;
  mkdirSync(resolve(dist, route), { recursive: true });
  writeFileSync(resolve(dist, route, 'index.html'), page(
    `${title} — Kessetsu`, `${title}. Public Kessetsu documentation for circuit simulation, executable requirements and schematic exports.`, route, body,
    { scripts: false, extraCss: `<link rel="stylesheet" href="${base}docs.css" />` },
  ));
}
const urls = ['', 'install/', ...toolPages.map(([route]) => route), ...routes.values()];
writeFileSync(resolve(dist, 'sitemap.xml'), `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${urls.map((route) => `  <url><loc>${origin}${route}</loc></url>`).join('\n')}\n</urlset>\n`);
console.log(`Public prerender PASS: landing, installation, ${toolPages.length} toolkit pages and ${documents.length} reviewed documentation pages; ${urls.length} sitemap URLs.`);
