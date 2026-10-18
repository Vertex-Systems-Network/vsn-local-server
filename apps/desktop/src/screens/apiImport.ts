// Import Postman (v2.0/v2.1) collections and OpenAPI 3 / Swagger 2 specs into API client requests.

export type ImportedRequest = {
  name: string;
  method: string;
  port: number;
  path: string;
  headers: Record<string, string>;
  body: string;
};

const METHODS = ['get', 'post', 'put', 'patch', 'delete', 'head', 'options'];

/** Safe file name for `.devify/api/<name>.json`. */
export function requestName(raw: string) {
  return (
    raw
      .replace(/[/{}]+/g, ' ')
      .replace(/[^A-Za-z0-9 _.-]+/g, '-')
      .replace(/ +/g, ' ')
      .replace(/-+/g, '-')
      .replace(/^[^A-Za-z0-9]+/, '')
      .slice(0, 64)
      .trim() || 'request'
  );
}

/** "{{baseUrl}}/users?x=1" or "http://localhost:3000/users" → "/users?x=1" (and the port when present). */
export function splitUrl(raw: string): { path: string; port?: number } {
  let rest = raw.trim();
  let port: number | undefined;
  const abs = rest.match(/^[a-z][a-z0-9+.-]*:\/\/([^/?#]*)(.*)$/i);
  if (abs) {
    const p = abs[1].match(/:(\d+)$/);
    if (p) port = Number(p[1]);
    rest = abs[2];
  } else if (rest.startsWith('{{')) {
    const end = rest.indexOf('}}');
    rest = end >= 0 ? rest.slice(end + 2) : rest;
  }
  if (!rest.startsWith('/')) rest = '/' + rest;
  return { path: rest, port };
}

type PostmanItem = {
  name?: string;
  item?: PostmanItem[];
  request?:
    | string
    | {
        method?: string;
        url?: string | { raw?: string; path?: string[] | string; query?: { key: string; value?: string }[] };
        header?: { key: string; value: string; disabled?: boolean }[];
        body?: { mode?: string; raw?: string };
      };
};

export function fromPostman(doc: { item?: PostmanItem[] }, defaultPort: number): ImportedRequest[] {
  const out: ImportedRequest[] = [];
  const walk = (items: PostmanItem[], prefix: string) => {
    for (const it of items) {
      const label = prefix ? `${prefix} - ${it.name ?? ''}` : (it.name ?? '');
      if (it.item) {
        walk(it.item, label);
        continue;
      }
      const r = typeof it.request === 'string' ? { url: it.request } : (it.request ?? {});
      let raw = '';
      if (typeof r.url === 'string') raw = r.url;
      else if (r.url?.raw) raw = r.url.raw;
      else if (r.url?.path) {
        const p = Array.isArray(r.url.path) ? r.url.path.join('/') : r.url.path;
        const q = (r.url.query ?? []).map((x) => `${x.key}=${x.value ?? ''}`).join('&');
        raw = `/${p}${q ? `?${q}` : ''}`;
      }
      const { path, port } = splitUrl(raw || '/');
      out.push({
        name: requestName(label || `${r.method ?? 'GET'} ${path}`),
        method: (r.method ?? 'GET').toUpperCase(),
        port: port ?? defaultPort,
        path,
        headers: Object.fromEntries(
          (r.header ?? []).filter((h) => !h.disabled).map((h) => [h.key.toLowerCase(), h.value]),
        ),
        body: r.body?.mode === 'raw' ? (r.body.raw ?? '') : '',
      });
    }
  };
  walk(doc.item ?? [], '');
  return out;
}

type OpenApiDoc = {
  openapi?: string;
  swagger?: string;
  servers?: { url: string }[];
  basePath?: string;
  paths?: Record<string, Record<string, { operationId?: string; summary?: string; requestBody?: unknown }>>;
};

export function fromOpenApi(doc: OpenApiDoc, defaultPort: number): ImportedRequest[] {
  const server = doc.servers?.[0]?.url ?? doc.basePath ?? '';
  const base = server ? splitUrl(server) : { path: '' };
  const prefix = base.path === '/' ? '' : base.path.replace(/\/$/, '');
  const out: ImportedRequest[] = [];
  for (const [p, ops] of Object.entries(doc.paths ?? {})) {
    for (const [m, op] of Object.entries(ops ?? {})) {
      if (!METHODS.includes(m)) continue;
      const path = prefix + p.replace(/\{([A-Za-z_][A-Za-z0-9_]*)\}/g, '{{$1}}');
      const hasBody = !!op?.requestBody || ['post', 'put', 'patch'].includes(m);
      out.push({
        name: requestName(op?.operationId || op?.summary || `${m.toUpperCase()} ${p}`),
        method: m.toUpperCase(),
        port: base.port ?? defaultPort,
        path,
        headers: hasBody
          ? { accept: 'application/json', 'content-type': 'application/json' }
          : { accept: 'application/json' },
        body: hasBody ? '{}' : '',
      });
    }
  }
  return out;
}

/** Detects the format and converts. Throws with a readable message for anything else. */
export function importRequests(text: string, defaultPort: number): ImportedRequest[] {
  let doc: unknown;
  try {
    doc = JSON.parse(text);
  } catch {
    throw new Error(
      'Paste JSON: a Postman collection or an OpenAPI/Swagger spec (YAML is not supported yet).',
    );
  }
  const d = doc as Record<string, unknown>;
  if (d && (d.openapi || d.swagger)) return fromOpenApi(d as OpenApiDoc, defaultPort);
  if (d && Array.isArray(d.item)) return fromPostman(d as { item: PostmanItem[] }, defaultPort);
  throw new Error('Not a Postman collection or OpenAPI spec.');
}
