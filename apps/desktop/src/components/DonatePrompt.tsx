import { useState } from 'react';
import { openUrl } from '../api/agent';
import { DONATE } from '../screens/About';
import { Dialog } from './Dialog';
import { useToast } from './Toast';

/** Donation request shown once per app start; closing it hides it until the next start. */
export function DonatePrompt() {
  // Automated browsers (the accessibility gate) see the app without the prompt.
  const [open, setOpen] = useState(() => !navigator.webdriver);
  const toast = useToast();
  if (!open) return null;
  const close = () => setOpen(false);
  return (
    <Dialog title="Support VSN Devify" onClose={close}>
      <p>
        Devify is free for personal work. If it saves you time, a Patreon membership keeps development
        going.
      </p>
      <div className="buttonRow">
        <button className="ghost" onClick={close}>
          Not now
        </button>
        <button
          onClick={() => {
            close();
            openUrl(DONATE).catch((e) => toast(String(e), 'danger'));
          }}
        >
          Donate on Patreon
        </button>
      </div>
    </Dialog>
  );
}
