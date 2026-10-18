# Sharing and Tunnels

**Networking → Share this app**

| Option | Who can open it | Needs |
| --- | --- | --- |
| Share on Wi-Fi / LAN | devices on the same private network, with a QR code | nothing |
| Share status page | phones on the LAN; read-only page of spaces and service status behind a secret link | nothing |
| Internet link — Cloudflare | anyone with the link | cloudflared |
| Internet link — ngrok | anyone with the link | ngrok + `ngrok config add-authtoken` |
| Internet link — frp | anyone with the link, through **your own** frps server | frpc; token kept in the vault |

Every share turns itself off after the time you choose (30 minutes – 24 hours). LAN shares refuse connections from outside private networks and limit connections to slow down abuse.
