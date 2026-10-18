import { Icon } from './Icon';

/** Overview when the authenticated agent bridge is unreachable (Figma: "33 State — Agent offline"). */
export function AgentOffline({
  error,
  lastSeen,
  busy,
  onRetry,
}: {
  error: string;
  lastSeen: Date | null;
  busy: boolean;
  onRetry: () => void;
}) {
  return (
    <>
      <section className="panel offlineHero" aria-labelledby="offline-title">
        <div className="offlineIcon" aria-hidden="true">
          <Icon name="server" size={24} />
        </div>
        <h2 id="offline-title">Can’t reach the VSN agent</h2>
        <p>
          The background agent is not running. VSN can start it for you; this takes a few seconds.
        </p>
        <button className="primary" onClick={onRetry} disabled={busy}>
          {busy ? 'Starting agent…' : 'Start agent'}
        </button>
      </section>
      <section className="panel" aria-labelledby="offline-trouble">
        <div className="panelHead">
          <h3 id="offline-trouble">Troubleshooting</h3>
        </div>
        <dl className="facts kvList">
          <div>
            <dt>Service</dt>
            <dd>VSNAgent</dd>
          </div>
          <div>
            <dt>Last seen</dt>
            <dd>{lastSeen ? lastSeen.toLocaleTimeString() : 'Not this session'}</dd>
          </div>
          <div>
            <dt>Last error</dt>
            <dd>
              <code>{error.replace(/^Agent unavailable:\s*/, '')}</code>
            </dd>
          </div>
        </dl>
      </section>
    </>
  );
}
