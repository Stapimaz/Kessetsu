import { ArrowLeft, Check, Copy, ExternalLink } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { BrandWordmark } from './BrandWordmark';
import '../landing.css';
import '../install.css';

type SetupMode = 'mcp' | 'cli';
type Platform = 'windows' | 'macos' | 'linux';
type ClientTab = 'claude-code' | 'cursor' | 'vscode' | 'claude-desktop' | 'windsurf';

const origin = 'https://kessetsu.com';
const verificationPrompt =
  'Check Kessetsu capabilities and simulate a 1 kHz RC low-pass filter (R1=1k, C1=159.15nF) to verify its -3 dB cutoff.';

const platforms: Record<Platform, { label: string; shell: string; command: string }> = {
  windows: {
    label: 'Windows',
    shell: 'PowerShell',
    command: `irm ${origin}/install.ps1 | iex`,
  },
  macos: {
    label: 'macOS',
    shell: 'Terminal',
    command: `curl -fsSL ${origin}/install.sh | sh`,
  },
  linux: {
    label: 'Linux',
    shell: 'Terminal',
    command: `curl -fsSL ${origin}/install.sh | sh`,
  },
};

const mcpServersConfig = '{\n  "mcpServers": {\n    "kessetsu": {\n      "command": "kess",\n      "args": ["mcp"]\n    }\n  }\n}';
const vscodeMcpConfig = '{\n  "servers": {\n    "kessetsu": {\n      "type": "stdio",\n      "command": "kess",\n      "args": ["mcp"]\n    }\n  }\n}';

const clientConfigs: Record<ClientTab, { label: string; commandLabel: string; command: string; pathHint: string }> = {
  'claude-code': {
    label: 'Claude Code',
    commandLabel: 'MCP registration command',
    command: 'claude mcp add kessetsu -- kess mcp',
    pathHint: 'Run in your terminal (registers in ~/.claude.json). Restart Claude Code if it was already open.',
  },
  cursor: {
    label: 'Cursor',
    commandLabel: 'Cursor MCP config',
    command: mcpServersConfig,
    pathHint: 'Save in .cursor/mcp.json (project) or ~/.cursor/mcp.json (global), then restart Cursor.',
  },
  vscode: {
    label: 'VS Code',
    commandLabel: 'VS Code MCP config',
    command: vscodeMcpConfig,
    pathHint: 'Save in .vscode/mcp.json in your workspace folder, then restart VS Code.',
  },
  'claude-desktop': {
    label: 'Claude Desktop',
    commandLabel: 'Claude Desktop MCP config',
    command: mcpServersConfig,
    pathHint: 'Settings → Developer → Edit Config (%APPDATA%\\Claude\\claude_desktop_config.json on Windows; ~/Library/Application Support/Claude/claude_desktop_config.json on macOS).',
  },
  windsurf: {
    label: 'Windsurf',
    commandLabel: 'Windsurf MCP config',
    command: mcpServersConfig,
    pathHint: 'Add to ~/.codeium/windsurf/mcp_config.json, then click Refresh or restart Windsurf.',
  },
};

const clientOrder: ClientTab[] = ['claude-code', 'cursor', 'vscode', 'claude-desktop', 'windsurf'];

function initialPlatform(): Platform {
  const agent = globalThis.navigator.userAgent;
  if (/Windows/i.test(agent)) return 'windows';
  if (/Macintosh|Mac OS X/i.test(agent)) return 'macos';
  return 'linux';
}

function CopyCommand({ command, label }: { command: string; label: string }) {
  const [state, setState] = useState<'idle' | 'copied' | 'manual'>('idle');
  const currentCommand = useRef(command);
  currentCommand.current = command;
  useEffect(() => { setState('idle'); }, [command]);
  useEffect(() => {
    if (state !== 'copied') return;
    const timeout = globalThis.setTimeout(() => setState('idle'), 2000);
    return () => globalThis.clearTimeout(timeout);
  }, [state]);
  return (
    <div className="install-command-wrap">
      <div className="install-command">
        <code>{command}</code>
        <button
          type="button"
          aria-label={`Copy ${label}`}
          onClick={async () => {
            try {
              await navigator.clipboard.writeText(command);
              if (currentCommand.current === command) setState('copied');
            } catch {
              if (currentCommand.current === command) setState('manual');
            }
          }}
        >
          {state === 'copied' ? <Check size={15} /> : <Copy size={15} />}
          <span>{state === 'copied' ? 'Copied' : 'Copy'}</span>
        </button>
      </div>
      {state === 'manual' && (
        <span className="install-copy-status" role="status">
          Select the command above and copy it manually.
        </span>
      )}
    </div>
  );
}

export function InstallPage() {
  // Keep build-time HTML independent of the build machine's navigator and URL hash.
  const [mode, setMode] = useState<SetupMode>('mcp');
  const [platform, setPlatform] = useState<Platform>('windows');
  const [clientTab, setClientTab] = useState<ClientTab>('claude-code');
  const selected = platforms[platform];
  const selectedClient = clientConfigs[clientTab];
  const base = import.meta.env.BASE_URL;

  useEffect(() => {
    setPlatform(initialPlatform());
    document.documentElement.dataset.theme = 'dark';
    document.title = 'Install Kessetsu CLI — Kessetsu';

    const syncModeFromHash = () => {
      const hash = globalThis.location.hash;
      if (hash === '#cli-verify') setMode('cli');
      else if (hash === '#mcp-setup') setMode('mcp');
    };
    syncModeFromHash();
    globalThis.addEventListener('hashchange', syncModeFromHash);
    return () => globalThis.removeEventListener('hashchange', syncModeFromHash);
  }, []);

  useEffect(() => {
    const hash = globalThis.location.hash;
    if ((mode === 'cli' && hash === '#cli-verify') || (mode === 'mcp' && hash === '#mcp-setup')) {
      document.getElementById(hash.slice(1))?.scrollIntoView();
    }
  }, [mode]);

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
        <p className="eyebrow">CLI &amp; MCP SETUP</p>
        <h1 id="install-title">Install Kessetsu</h1>
        <p className="install-lead">
          One command installs the local <code>kess</code> CLI and built-in MCP server.{' '}
          <a href={`${base}#editor`}>No install needed for the browser editor — Open Web Hub →</a>
        </p>

        <div className="install-controls">
          <div className="install-mode-tabs" role="tablist" aria-label="Setup mode">
            <button
              type="button"
              role="tab"
              id="tab-mode-mcp"
              aria-selected={mode === 'mcp'}
              aria-labelledby="tab-mode-mcp-title"
              aria-describedby="tab-mode-mcp-desc"
              onClick={() => setMode('mcp')}
            >
              <span id="tab-mode-mcp-title" className="install-mode-title">MCP (AI Agents)</span>
              <span id="tab-mode-mcp-desc" className="install-mode-desc">Claude Code, Cursor, VS Code, Windsurf</span>
            </button>
            <button
              type="button"
              role="tab"
              id="tab-mode-cli"
              aria-selected={mode === 'cli'}
              aria-labelledby="tab-mode-cli-title"
              aria-describedby="tab-mode-cli-desc"
              onClick={() => setMode('cli')}
            >
              <span id="tab-mode-cli-title" className="install-mode-title">CLI (Terminal &amp; CI)</span>
              <span id="tab-mode-cli-desc" className="install-mode-desc">kess check, test, study &amp; export</span>
            </button>
          </div>

          <div className="install-platforms" role="tablist" aria-label="Operating system">
            {(Object.keys(platforms) as Platform[]).map((key) => (
              <button
                key={key}
                type="button"
                role="tab"
                aria-selected={key === platform}
                onClick={() => setPlatform(key)}
              >
                {platforms[key].label}
              </button>
            ))}
          </div>
        </div>

        <ol className="install-steps">
          <li>
            <div className="install-step-header">
              <h2>Install kess</h2>
              <span className="install-step-badge">{selected.shell}</span>
            </div>
            <CopyCommand command={selected.command} label="installation command" />
            {platform === 'windows' && (
              <p className="install-note">Includes Kessetsu CLI, MCP server, and Ngspice 46 — no admin rights needed.</p>
            )}
            {platform === 'macos' && (
              <p className="install-note">
                Includes Kessetsu CLI and MCP server. For simulation, also install Ngspice: <code>brew install ngspice</code>.
              </p>
            )}
            {platform === 'linux' && (
              <p className="install-note">
                Includes Kessetsu CLI and MCP server. For simulation, also install Ngspice: <code>sudo apt install ngspice</code>.
              </p>
            )}
          </li>

          {mode === 'mcp' ? (
            <>
              <li id="mcp-setup">
                <div className="install-step-header">
                  <h2>Register the MCP server</h2>
                </div>
                <div className="install-client-tabs" role="tablist" aria-label="Choose your AI application">
                  {clientOrder.map((tabId) => (
                    <button
                      key={tabId}
                      type="button"
                      role="tab"
                      aria-selected={tabId === clientTab}
                      onClick={() => setClientTab(tabId)}
                    >
                      {clientConfigs[tabId].label}
                    </button>
                  ))}
                </div>
                <div className="install-client-panel" role="tabpanel" aria-label={selectedClient.label}>
                  <CopyCommand command={selectedClient.command} label={selectedClient.commandLabel} />
                  <p className="install-note">{selectedClient.pathHint}</p>
                </div>
              </li>

              <li>
                <div className="install-step-header">
                  <h2>Verify in your AI chat</h2>
                </div>
                <CopyCommand command={verificationPrompt} label="verification prompt" />
                <p className="install-note">
                  Paste this prompt into your AI chat to confirm Ngspice and requirements work end-to-end.{' '}
                  <a href={`${base}docs/guides/agent-workflow/`}>Agent design workflow →</a>
                </p>
              </li>
            </>
          ) : (
            <li id="cli-verify">
              <div className="install-step-header">
                <h2>Verify in your terminal</h2>
              </div>
              <CopyCommand command="kess --version" label="version command" />
              <p className="install-note">
                If an already-open terminal or IDE cannot find <code>kess</code>, open a new session to pick up the updated PATH.
              </p>
              <details className="install-inline-details">
                <summary>Try a 30-second RC low-pass check in the terminal</summary>
                <div className="install-details-body">
                  <p>
                    <a href={`${base}examples/rc_low_pass.kess`} download="my-circuit.kess">Download the RC low-pass example</a>{' '}
                    as <code>my-circuit.kess</code>, then run from its folder:
                  </p>
                  <CopyCommand command="kess check my-circuit.kess" label="first circuit command" />
                  <CopyCommand command="kess test my-circuit.kess" label="simulation command" />
                  <CopyCommand command="kess test my-circuit.kess --format json" label="JSON CLI test command" />
                  <p className="install-note">
                    Five electrical assertions should PASS. See the <a href={`${base}docs/reference/cli/`}>CLI reference</a> or{' '}
                    <a href={`${base}docs/guides/troubleshooting/`}>troubleshooting guide</a>.
                  </p>
                </div>
              </details>
            </li>
          )}
        </ol>

        <section className="install-secondary" aria-label="Additional installation options">
          <details>
            <summary>Prefer not to pipe to a shell? Inspect or install manually</summary>
            <div className="install-details-body">
              <p>
                Read the script before running it:{' '}
                <a href={`${base}${platform === 'windows' ? 'install.ps1' : 'install.sh'}`}>
                  view {platform === 'windows' ? 'install.ps1' : 'install.sh'}
                </a>
                . The installer verifies SHA-256 release hashes before placing binaries in your user directory.
              </p>
              <p>
                Or <a href="https://github.com/Stapimaz/Kessetsu/releases/latest">download release packages manually <ExternalLink size={12} /></a>{' '}
                and add the <code>bin</code> folder to your user PATH.
              </p>
            </div>
          </details>

          <details>
            <summary>Update, uninstall, or fix PATH issues</summary>
            <div className="install-details-body">
              <p>
                <strong>Update:</strong> Run the Step 1 install command again. Failed downloads or hash checks never overwrite your working CLI.
              </p>
              <p>
                <strong>PATH issues:</strong> Open a new terminal (or restart your IDE/AI app) after installing. Still stuck? See{' '}
                <a href={`${base}docs/guides/troubleshooting/`}>Troubleshooting</a>.
              </p>
              <p>
                <strong>Uninstall:</strong> Close terminals and agents using Kessetsu.{' '}
                {platform === 'windows' ? (
                  <>
                    In File Explorer, open <code>%LOCALAPPDATA%</code> and remove only the <code>Kessetsu</code> folder, then remove{' '}
                    <code>%LOCALAPPDATA%\Kessetsu\bin</code> from your user Path.
                  </>
                ) : (
                  <>
                    Remove only the <code>kess</code> symlink in <code>~/.local/bin</code> and the <code>~/.local/share/kessetsu</code> folder.
                    Do not delete the surrounding bin/share folders. Remove only the line ending <code># Kessetsu installer</code> from your
                    shell startup file (<code>.bashrc</code> or <code>.zshrc</code>). System Ngspice is separate and is not removed.
                  </>
                )}
              </p>
            </div>
          </details>
        </section>
      </section>
    </main>
  );
}
