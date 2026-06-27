# Deploying RavenClaws.io

> **Canonical reference:** [`docs/guides/website.md`](../docs/guides/website.md) in the repo root contains the
> full deployment walkthrough, content management guide, and AI agent instructions.
> This file is a local copy for quick reference.

This folder is a self-contained static site (no build step) for **https://ravenclaws.io**,
deployed to **Cloudflare** using **Workers Static Assets** via **Wrangler**.

Everything the browser needs lives in [`public/`](./public). Cloudflare uploads that
directory to its edge and serves it directly — there is no bundler, framework, or
runtime. This mirrors RavenClaws itself: small, simple, zero dependencies.

```
website/
├── public/                 # ← everything served at ravenclaws.io
│   ├── index.html          # landing page
│   ├── 404.html
│   ├── docs/               # documentation hub
│   │   ├── index.html
│   │   ├── getting-started.html
│   │   ├── configuration.html
│   │   ├── swarm-mode.html
│   │   ├── mcp-integration.html
│   │   └── heartbeat-mode.html
│   ├── assets/             # css, js, svg art, favicon, og image
│   ├── _headers            # security + cache headers
│   ├── _redirects          # shortlinks (/github, /crate, …)
│   ├── robots.txt
│   └── sitemap.xml
├── wrangler.jsonc          # Cloudflare deploy config
├── package.json            # wrangler dev-dependency + scripts
└── DEPLOY.md               # this file
```

---

## 1. Prerequisites

- A **Cloudflare account** (free tier is fine).
- **Node.js 18+** (only to run Wrangler — the site itself ships no JS toolchain).
- The **ravenclaws.io** domain. The simplest path is to add the zone to Cloudflare
  (Cloudflare dashboard → *Add a site*) so the custom domain can attach automatically.

Install dependencies once:

```bash
cd website
npm install
```

---

## 2. Local preview

Two options:

```bash
# Cloudflare-accurate preview (serves exactly like production, honours _headers/_redirects)
npm run dev          # → http://localhost:8787

# Or a plain static server (no Cloudflare features)
npm run preview      # → http://localhost:3000
```

---

## 3. First deploy (manual)

Authenticate Wrangler with your Cloudflare account, then deploy:

```bash
cd website
npx wrangler login          # opens a browser to authorize
npm run deploy              # = wrangler deploy
```

Wrangler uploads `public/` and prints your `*.workers.dev` URL. Open it to confirm
the site is live before attaching the custom domain.

---

## 4. Attach the custom domain (ravenclaws.io)

Custom domains are **deliberately not** in `wrangler.jsonc`. Attaching them with a
`routes` + `custom_domain` block during a Workers Builds deploy fails the whole build
if anything is off (zone not in the account, a conflicting apex/`www` DNS record, or a
build token without Zone-DNS permission). Attach them from the dashboard instead —
it succeeds independently of the deploy and tells you the exact conflict:

> Cloudflare dashboard → **Workers & Pages** → **ravenclaws-website** → **Settings** →
> **Domains & Routes** → **Add** → **Custom domain** → `ravenclaws.io`

Repeat for `www.ravenclaws.io`. Cloudflare creates the DNS records and TLS certificate.

**Prerequisites for the custom domain to attach:**

1. **The `ravenclaws.io` zone must be in this Cloudflare account** (dashboard →
   *Add a domain*) with its nameservers pointing at Cloudflare. A Workers custom
   domain can only be created for a zone Cloudflare manages.
2. **No conflicting records.** If an apex `A`/`AAAA`/`CNAME` (or a `www` record)
   already exists, remove it first — the custom domain needs to create its own.
3. If you ever want Wrangler to manage it again, re-add the `routes` block and ensure
   the deploy token has **Zone → DNS → Edit** for the zone.

---

## 5. Redeploying

Whenever you change anything under `public/`, redeploy with one command:

```bash
cd website
npm run deploy        # = wrangler deploy
```

Wrangler diffs the asset manifest and uploads only what changed. That's the whole
workflow — no build step, no CI, no pipeline.

> **Prefer Cloudflare Pages?** The same `public/` folder deploys unchanged with
> `npx wrangler pages deploy public --project-name ravenclaws`. Use whichever
> of **Workers** or **Pages** you like in the Cloudflare dashboard — both are driven
> by Wrangler.

---

## 6. Updating content

- **Landing page** — edit `public/index.html`.
- **Docs** — edit the matching file in `public/docs/`. The pages mirror the guides in
  the repo's [`docs/guides/`](../docs/guides). When you update a guide, update its page.
- **Styling** — `public/assets/styles.css` (single theme used everywhere).
- **Art** — `public/assets/raven-*.webp` (plus favicons and `og-image.png`). These were
  derived from the source artwork in `~/Downloads/RavenClaws/` (backgrounds removed,
  resized, and optimized to WebP). To swap in a different image, drop it in
  `public/assets/` and update the matching `<img src>` in the HTML.

---

## Notes

- **Why Workers Static Assets and not Pages?** Cloudflare now recommends Workers (with
  static assets) for new projects; Workers Sites is deprecated in Wrangler v4. The
  same `public/` folder also deploys to Cloudflare Pages unchanged
  (`npx wrangler pages deploy public`) if you ever prefer Pages.
- **Security headers** are set in `public/_headers` (HSTS, CSP, `X-Content-Type-Options`,
  frame-deny, a tight referrer policy). Adjust the CSP if you add third-party embeds.
- The site is fully static and has **no analytics or phone-home** — consistent with the
  project's "no telemetry, ever" stance.
