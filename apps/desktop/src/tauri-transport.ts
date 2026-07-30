/**
 * Transport for the desktop build.
 *
 * Same interface as the WebSocket transport, but frames go straight to Rust
 * over Tauri IPC. There is no network hop, no handshake and no authentication
 * to do: only this window can reach these commands.
 */

import { BaseTransport, type ClientMessage } from '@majin/protocol';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

interface PtyReady {
  shell: string;
  cols: number;
  rows: number;
}

/** Geometry the PTY starts at, replaced as soon as the terminal fits itself. */
const INITIAL_SIZE = { cols: 80, rows: 24 };

export class TauriTransport extends BaseTransport {
  #unlisteners: UnlistenFn[] = [];
  #open = false;
  #disposed = false;

  connect(): void {
    if (this.#disposed || this.#open) return;
    this.setState({ status: 'connecting' });
    void this.#start();
  }

  async #start(): Promise<void> {
    try {
      this.#unlisteners.push(
        await listen<string>('pty://output', (event) => {
          this.emitMessage({ type: 'output', data: event.payload });
        }),
        await listen<number>('pty://exit', (event) => {
          this.emitMessage({ type: 'exit', exitCode: event.payload });
          this.setState({ status: 'closed', detail: 'The shell exited.' });
          this.#open = false;
        }),
      );

      if (this.#disposed) return;

      const ready = await invoke<PtyReady>('pty_open', INITIAL_SIZE);
      this.#open = true;
      this.setState({ status: 'open' });
      this.emitMessage({
        type: 'ready',
        protocolVersion: 1,
        sessionId: 'local',
        shell: ready.shell,
        cols: ready.cols,
        rows: ready.rows,
      });
    } catch (cause) {
      this.setState({
        status: 'error',
        detail: cause instanceof Error ? cause.message : String(cause),
      });
      this.emitMessage({
        type: 'error',
        code: 'spawn_failed',
        message: cause instanceof Error ? cause.message : String(cause),
      });
    }
  }

  send(message: ClientMessage): void {
    if (this.#disposed) return;

    switch (message.type) {
      case 'input':
        if (this.#open) void invoke('pty_write', { data: message.data }).catch(noop);
        break;
      case 'resize':
        // Sent even before the shell is up: Rust ignores a resize with no
        // session, and the terminal re-sends after `ready`.
        void invoke('pty_resize', { cols: message.cols, rows: message.rows }).catch(noop);
        break;
      case 'ping':
        this.emitMessage({ type: 'pong', at: message.at });
        break;
    }
  }

  override dispose(): void {
    this.#disposed = true;
    this.#open = false;
    for (const unlisten of this.#unlisteners) unlisten();
    this.#unlisteners = [];
    void invoke('pty_close').catch(noop);
    super.dispose();
  }
}

function noop() {
  // Command failures after teardown are expected and not worth reporting.
}
