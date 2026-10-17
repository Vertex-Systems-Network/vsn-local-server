import { useEffect, useState } from 'react';
import { call } from '../api/agent';

type AppConfig = {
  version: number;
  workspace_roots: string[];
  default_domain_suffix: string;
  default_execution_backend: string;
  telemetry_enabled: boolean;
};

/** Read-only view of the agent's config.json (Figma: Settings → Advanced). */
export function AgentConfig() {
  const [cfg, setCfg] = useState<AppConfig | null>(null);
  const [err, setErr] = useState('');
  useEffect(() => {
    call<AppConfig>('config.show')
      .then(setCfg)
      .catch((e) => setErr(String(e)));
  }, []);
  return (
    <section className="panel" aria-labelledby="cfg-title">
      <div className="panelHead">
        <div>
          <span className="kicker">Agent</span>
          <h3 id="cfg-title">Configuration</h3>
        </div>
      </div>
      {cfg ? (
        <dl className="facts kvList">
          <div>
            <dt>Local domain suffix</dt>
            <dd>
              <code>{cfg.default_domain_suffix}</code>
            </dd>
          </div>
          <div>
            <dt>Execution backend</dt>
            <dd>{cfg.default_execution_backend}</dd>
          </div>
          <div>
            <dt>Workspace roots</dt>
            <dd>{cfg.workspace_roots.length}</dd>
          </div>
          <div>
            <dt>Telemetry</dt>
            <dd>{cfg.telemetry_enabled ? 'On' : 'Off'}</dd>
          </div>
          <div>
            <dt>Config version</dt>
            <dd>{cfg.version}</dd>
          </div>
        </dl>
      ) : (
        <p className="hint">{err || 'Loading…'}</p>
      )}
      <p className="hint">Values are read from the agent. Editing them from the app is not supported yet.</p>
    </section>
  );
}
