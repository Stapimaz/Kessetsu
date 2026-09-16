import { ArrowLeft, Check, Copy, ExternalLink, Terminal } from 'lucide-react';
import { useEffect, useState } from 'react';
import { BrandWordmark } from './BrandWordmark';
import '../landing.css';
import '../install.css';

type Platform = 'windows' | 'macos' | 'linux';
const origin = 'https://kessetsu.com';
const platforms: Record<Platform, { label: string; terminal: string; command: string }> = {
  windows: {
    label: 'Windows',
    terminal: 'Open Start, type PowerShell, and open Windows PowerShell. You do not need “Run as administrator”.',
    command: `irm ${origin}/install.ps1 | iex`,
  },
  macos: {
    label: 'macOS',
    terminal: 'Press Command + Space, type Terminal, and press Enter. Both Intel and Apple Silicon are detected automatically.',
    command: `curl -fsSL ${origin}/install.sh | sh`,
  },
  linux: {
    label: 'Linux',
    terminal: 'Open your Terminal app (usually Ctrl + Alt + T). The ready-to-use Linux package supports x86-64.',
    command: `curl -fsSL ${origin}/install.sh | sh`,
  },
};

function initialPlatform(): Platform {
  const agent = globalThis.navigator.userAgent;
  if (/Windows/i.test(agent)) return 'windows';
  if (/Macintosh|Mac OS X/i.test(agent)) return 'macos';
  return 'linux';
}

function CopyCommand({ command, label }: { command: string; label: string }) {
  const [state, setState] = useState<'idle' | 'copied' | 'manual'>('idle');
  useEffect(() => { setState('idle'); }, [command]);
  useEffect(() => {
    if (state !== 'copied') return;
    const timeout = globalThis.setTimeout(() => setState('idle'), 2000);
    return () => globalThis.clearTimeout(timeout);
  }, [state]);
  return (
    <div>
      <div className="install-command">
        <code>{command}</code>
        <button type="button" aria-label={`Copy ${label}`} onClick={async () => {
          try { await navigator.clipboard.writeText(command); setState('copied'); }
          catch { setState('manual'); }
        }}>{state === 'copied' ? <Check size={18} /> : <Copy size={18} />}</button>
      </div>
      <span className="install-copy-status" role="status">{state === 'copied' ? 'Copied' : state === 'manual' ? 'Select the command above and copy it manually.' : ''}</span>
    </div>
  );
}

export function InstallPage() {
  // Keep build-time HTML independent of the build machine's navigator.
  const [platform, setPlatform] = useState<Platform>('windows');
  const selected = platforms[platform];
  const base = import.meta.env.BASE_URL;
  useEffect(() => {
    setPlatform(initialPlatform());
    document.documentElement.dataset.theme = 'dark';
    document.title = 'Install Kessetsu CLI — Kessetsu';
  }, []);

  return (
    <main className="landing-page install-page">
      <header className="landing-header">
        <a className="landing-wordmark" href={base} aria-label="Kessetsu home"><BrandWordmark /></a>
        <nav className="landing-nav" aria-label="Installation navigation">
          <a href={`${base}#editor`}>Web Hub</a>
          <a href={`${base}docs/`}>Docs</a>
        </nav>
      </header>
      <section className="install-content" aria-labelledby="install-title">
        <a className="text-link install-back" href={base}><ArrowLeft size={14} /> Back to Kessetsu</a>
        <p className="eyebrow">CLI quickstart</p>
        <h1 id="install-title">A few steps. Your first circuit.</h1>
        <p className="install-lead">Install Kessetsu once. Use <code>kess</code> from any folder, yourself or through your AI agent. Free, local, and no account required.</p>
        <div className="install-platforms" role="group" aria-label="Choose your operating system">
          {(Object.keys(platforms) as Platform[]).map((key) => (
            <button key={key} type="button" aria-pressed={key === platform} onClick={() => setPlatform(key)}>{platforms[key].label}</button>
          ))}
        </div>
        <ol className="install-steps">
          <li>
            <h2>Open your terminal</h2><p>{selected.terminal}</p>
            <p className="install-note">A terminal is just where you paste the commands below. You do not need to write code to install.</p>
          </li>
          <li>
            <h2>Paste one command</h2><p>Copy this into {platform === 'windows' ? 'PowerShell' : 'Terminal'}, then press Enter.</p>
            <CopyCommand command={selected.command} label="installation command" />
            <p className="install-note">The installer selects the latest stable release, checks its SHA-256 hashes, and sets up your user PATH. No manual ZIP extraction or administrator access is needed.</p>
          </li>
          <li>
            <h2>Open a new terminal and check</h2><p>Close the terminal and open a new one so it picks up the installation. If you use a VS Code terminal, restart VS Code.</p>
            <CopyCommand command="kess --version" label="version command" />
            <p>You should see <code>kess</code> followed by the installed version number.</p>
          </li>
          <li>
            <h2>Try your first circuit</h2>
            <p><a href={`${base}examples/rc_low_pass.kess`} download="my-circuit.kess">Download the RC low-pass example</a> as <code>my-circuit.kess</code>. Open a terminal in the folder where you saved it, then run:</p>
            <p className="install-note">{platform === 'windows' ? 'In File Explorer, open that folder, right-click an empty area, and choose Open in Terminal (on older Windows, hold Shift and choose Open PowerShell window here).' : 'In Terminal, type cd followed by a space, drag the saved file’s folder onto the terminal, and press Enter.'}</p>
            <CopyCommand command="kess check my-circuit.kess" label="first circuit command" />
            <p>For simulation and the example’s electrical requirements:</p>
            <CopyCommand command="kess test my-circuit.kess" label="simulation command" />
            {platform === 'windows' ? <p className="install-note">Ngspice, the simulator, is included in the Windows installation.</p> : (
              <div className="install-prerequisite">
                <h3>Simulation needs Ngspice</h3>
                <p>Compile, check, and export work immediately. Before <code>kess test</code>, install the simulator if you do not already have it:</p>
                <CopyCommand command={platform === 'macos' ? 'brew install ngspice' : 'sudo apt-get update && sudo apt-get install ngspice'} label="simulator command" />
                <p className="install-note">{platform === 'macos' ? <>This command needs <a href="https://brew.sh/">Homebrew</a>. If it is not installed, follow its official installer first.</> : 'This command is for Ubuntu/Debian. On other Linux distributions, install ngspice with your package manager.'} The Kessetsu installer does not run privileged commands for you.</p>
              </div>
            )}
          </li>
        </ol>
        <section className="install-followup" aria-labelledby="install-next">
          <h2 id="install-next"><Terminal size={20} /> Use it with your agent</h2>
          <p>Tell your agent: “Use the installed <code>kess</code> CLI to design, simulate, test, and export my circuit. Start with <code>kess --help</code> and return structured results using <code>--format json</code>.” Your agent stays yours; Kessetsu does not require an AI subscription or API key.</p>
          <h2>Updates and alternatives</h2>
          <p>To update, run the same installation command again. A failed download or hash check will not replace your working CLI. Previous bundles are retained for recovery.</p>
          <p>The command executes a script from this website. If you prefer to inspect it first: <a href={`${base}${platform === 'windows' ? 'install.ps1' : 'install.sh'}`}>view the installer</a>. Checksums detect damaged downloads; they are not publisher signatures.</p>
          <p><a href="https://github.com/Stapimaz/Kessetsu/releases/latest">Download packages manually <ExternalLink size={12} /></a> · <a href="https://github.com/Stapimaz/Kessetsu/blob/main/docs/guides/troubleshooting.md">Troubleshooting</a></p>
        </section>
      </section>
    </main>
  );
}
