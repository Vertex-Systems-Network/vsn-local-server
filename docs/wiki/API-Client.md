# API Client

A Postman-style client per project. Requests are saved as plain files in `<space>/.devify/api/` so they can be committed.

- `{{VAR}}` placeholders are filled from the space's environment.
- **Auth helpers**: Bearer token and Basic auth.
- **Tests**: simple checks after every send — expected status, text the body must contain, and a maximum response time.
- **History** of recent requests.
- **Import** Postman collections and OpenAPI specs.
