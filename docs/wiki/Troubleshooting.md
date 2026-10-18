# Troubleshooting

## Diagnose
**System → Help → Diagnose** runs a read-only checklist and shows, for every item, **OK** or what is wrong and how to fix it:
- the VSN Agent and its data folder,
- the audit log chain,
- your workspace folders,
- every space service: is its port answering?
- optional tools (git, svn, php, composer, node, caddy, mailpit, redis, rclone, curl, tar, cloudflared, ngrok, frpc) and what each one unlocks.

**Copy report** puts the result on the clipboard to send to support.

## Common problems
| Problem | Fix |
| --- | --- |
| App shows *Starting the VSN Agent…* for long | Wait 10 s; if it stays, restart the app. Antivirus may block `vsn-agent` — allow it. |
| `os error 10060` / timeouts | The agent was busy; the app retries. If it repeats, run Diagnose. |
| A service will not start | Diagnose → install the missing program, or **Change port** if another app uses it. |
| `name.test` does not open | Install Caddy and press **Route to port** again; accept the admin prompt. |
| Space actions say *locked* | Unlock the space in Spaces. |
| Cloud backup fails | Check the account under Backups → Cloud accounts; reconnect it. |
| Email backup fails | Use an **app password** (Gmail/Outlook); archives over 20 MB need another destination. |
| Internet link fails | Install cloudflared / ngrok / frpc; for ngrok add your authtoken. |

Still stuck? Open an issue with the Diagnose report attached.
