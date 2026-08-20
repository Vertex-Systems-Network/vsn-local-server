# Marketplace channels — 0.19

Signed marketplace entries can advertise explicit channels such as `stable`, `beta` or `nightly`. `resolve-update-channel` considers only newer, non-revoked versions that explicitly belong to the requested channel.

Channel membership is part of the signed marketplace index, so clients do not infer a prerelease channel purely from version text.
