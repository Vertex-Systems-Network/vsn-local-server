import { Badge } from '../components/ui';

export function Updates() {
  return (
    <section className="panel">
      <div className="panelHead">
        <div>
          <span className="kicker">Personal channel</span>
          <h3>Updates</h3>
        </div>
        <Badge ok label={`v${__APP_VERSION__}`} />
      </div>
      <ol className="steps">
        <li>
          On GitHub, open <b>Actions → Personal Windows Build (unsigned)</b> and click <b>Run workflow</b>.
        </li>
        <li>
          Download the <code>vsn-windows-unsigned-…</code> artifact and check its hash against <code>SHA256SUMS.txt</code>.
        </li>
        <li>Run the new installer over this one. Projects, databases and settings are kept.</li>
      </ol>
      <p className="hint">
        In-app automatic updates arrive with the signed public release (PKG-03 03.22–03.25). Full guide:{' '}
        <code>docs/PERSONAL-INSTALL-WINDOWS.md</code>.
      </p>
    </section>
  );
}
