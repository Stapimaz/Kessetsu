import { Component, type ReactNode } from 'react';
import './PageLoadBoundary.css';

/** Keeps a failed lazy page from replacing the whole site with a blank screen. */
export class PageLoadBoundary extends Component<{ children: ReactNode; page: string; onDismiss?: () => void }, { failed: boolean }> {
  state = { failed: false };

  static getDerivedStateFromError() { return { failed: true }; }

  render() {
    if (!this.state.failed) return this.props.children;
    const Surface = this.props.onDismiss ? 'aside' : 'main';
    return <Surface className={`page-load-error${this.props.onDismiss ? ' feature-load-error' : ''}`} role="alert">
      <h1>Could not open {this.props.page}</h1>
      <p>A page download or application error interrupted loading. Check your connection, then reload to try again.</p>
      <p>Reloading does not clear browser drafts. If you were using local model files, select them again after reopening the editor.</p>
      <div>{this.props.onDismiss && <button onClick={this.props.onDismiss}>Return to editor</button>}<button onClick={() => location.reload()}>Reload page</button>{!this.props.onDismiss && <a href={import.meta.env.BASE_URL}>Back to home</a>}<a href={`${import.meta.env.BASE_URL}docs/guides/troubleshooting/`}>Troubleshooting</a></div>
    </Surface>;
  }
}
