# Backups

**Connect → Backups** backs up a whole space: the project folder (without `node_modules`, `vendor`, `.git`, build output and live service data) plus fresh database dumps.

## Destinations
| Destination | How to connect |
| --- | --- |
| Folder on this PC, USB disk or a synced folder | full path; *Keep newest* deletes older archives |
| Email (Gmail, Outlook, any SMTP) | SMTP server, login and app password (stored in the vault); archives up to 20 MB |
| Google Drive, OneDrive, Dropbox, Box, pCloud | **Connect** opens the browser to sign in |
| MEGA | email and password |
| Amazon S3, Wasabi, Cloudflare R2, MinIO, Backblaze B2 | access key and secret |
| WebDAV / Nextcloud / ownCloud, SFTP, FTP | URL or host, user and password |

**Encrypted backups:** connect *Encrypted (password-protected layer)* on top of any account (for example `gdrive:VSN-encrypted`) and use it as the destination. Archive contents and file names are encrypted on this PC before upload; without the password nobody — including the cloud provider — can read them, and a lost password cannot be recovered.

Cloud accounts use **rclone** (`winget install Rclone.Rclone`); keys and passwords are stored obscured in VSN's private rclone config.

## Schedules
Pick a space and destination and press **Schedule** (hourly, every 6 hours, daily, weekly). The agent checks every five minutes; failures are shown on the schedule.

## Restore
Paste an archive path (`D:\Backups\shop\shop-….tar.gz`) or a cloud path (`gdrive:Backups/VSN/shop/shop-….tar.gz`), or press **Use for restore** in the history. VSN extracts it into a **new folder** beside the space — nothing is overwritten. Database dumps are in `.devify/backups/` of the restored copy.

> Archives include files such as `.env`; only use destinations you trust.
