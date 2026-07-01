# Deploying RavenClaws.io

> **AI agent instructions:** The `AGENTS.md` file in the repo root contains a
> comprehensive **Website (ravenclaws.io)** section with architecture, content
> management, common tasks, and guardrails for AI agents working on the website.
> This file is the deployment walkthrough.

This folder is a self-contained static site (no build step) for **https://ravenclaws.io**,
deployed to **Cloudflare** using **Workers Static Assets** via **Wrangler**.

**Auto-deploy:** Cloudflare is connected to the GitHub repository. Pushing changes to
the `master` branch automatically deploys `website/public/` to the edge. No manual
`npm run deploy` needed for routine updates.

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

**Option A — from `wrangler.jsonc` (already configured).**
The `routes` block in `wrangler.jsonc` claims `ravenclaws.io` and `www.ravenclaws.io`
as custom domains. This works automatically **if the domain's DNS is managed in the
same Cloudflare account**. Just `npm run deploy` and Cloudflare provisions the
records + TLS certificate.

**Option B — from the dashboard.**
If you'd rather wire it up by hand (or DNS lives elsewhere), delete the `routes`
block from `wrangler.jsonc`, deploy, then go to:

> Cloudflare dashboard → **Workers & Pages** → `ravenclaws-website` → **Settings** →
> **Domains & Routes** → **Add** → **Custom domain** → `ravenclaws.io`

Repeat for `www.ravenclaws.io`. Cloudflare creates the DNS records and certificate.

---

## 5. Redeploying

The website is **auto-deployed by Cloudflare** via its Git integration. When changes
are pushed to the `master` branch on GitHub, Cloudflare automatically pulls the
`website/public/` directory and deploys it to the edge. No manual steps required.

**Emergency manual deploy** (if Git integration is down):

```bash
cd website
npm run deploy        # = wrangler deploy
```

Wrangler diffs the asset manifest and uploads only what changed.

> **Prefer Cloudflare Pages?** The same `public/` folder deploys unchanged with
> `npx wrangler pages deploy public --project-name ravenclaws-website`. Use whichever
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