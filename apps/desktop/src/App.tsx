import type { TransportState } from '@majin/protocol';
import { TerminalView } from '@majin/terminal-ui';
import { useEffect, useMemo, useState } from 'react';
import { TauriTransport } from './tauri-transport';

export function App() {
  const [state, setState] = useState<TransportState>({ status: 'idle' });
  const [shell, setShell] = useState<string | null>(null);
  const [exitCode, setExitCode] = useState<number | null>(null);

  // One PTY per window; `sessionKey` bumps to start a fresh one after exit.
  const [sessionKey, setSessionKey] = useState(0);
  // biome-ignore lint/correctness/useExhaustiveDependencies: sessionKey is the restart signal.
  const transport = useMemo(() => new TauriTransport(), [sessionKey]);

  useEffect(() => () => transport.dispose(), [transport]);

  function restart() {
    setExitCode(null);
    setShell(null);
    setSessionKey((key) => key + 1);
  }

  return (
    <div className="app">
      {/* Space for the overlaid macOS traffic lights, and a drag handle. */}
      <header className="titlebar" data-tauri-drag-region>
        <span className={`titlebar__dot titlebar__dot--${state.status}`} aria-hidden="true" />
        <span className="titlebar__title">{shell ?? 'majin'}</span>
      </header>

      <div className="app__terminal">
        <TerminalView
          transport={transport}
          onTransportState={setState}
          onReady={(message) => setShell(message.shell)}
          onExit={setExitCode}
        />
      </div>

      {exitCode !== null && (
        <footer className="restart">
          <span>Shell exited with code {exitCode}.</span>
          <button type="button" onClick={restart}>
            Start a new session
          </button>
        </footer>
      )}
    </div>
  );
}
