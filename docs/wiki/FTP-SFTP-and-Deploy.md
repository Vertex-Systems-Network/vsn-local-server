# FTP, SFTP and Deploy

**Deploy** keeps one transfer connection per space (FTP, FTPS or SFTP):
- the password is stored in the vault;
- SFTP host keys are pinned on first use and checked every time after;
- uploads and downloads stay inside the space folder; symlinks are never followed.

List, upload and download files from the Deploy screen.
