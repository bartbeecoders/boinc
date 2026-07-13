# Deploying the Boinc portal (boinc.hideterms.com)

The portal is a React + Vite app in `site/` (`npm run dev` to work on it,
`npm run build` → static output in `site/dist`). Hosting target: Cloudflare
Pages. Deploys run automatically from GitHub Actions
(`.github/workflows/deploy-site.yml`) on every push to `main` that touches
`site/`: CI builds the app and uploads `site/dist`.

## One-time setup

1. **Create the Pages project** (from a machine with wrangler authenticated):

   ```sh
   npx wrangler login                # once
   npx wrangler pages project create boinc-portal --production-branch=main
   cd site && npm ci && npm run build
   npx wrangler pages deploy site/dist --project-name=boinc-portal   # first deploy
   ```

2. **Custom domain.** In the Cloudflare dashboard: Pages → boinc-portal →
   Custom domains → add `boinc.hideterms.com`. Because hideterms.com is on
   Cloudflare, the CNAME record is created automatically; otherwise add:

   ```
   CNAME  boinc  boinc-portal.pages.dev
   ```

3. **CI secrets** (GitHub repo → Settings → Secrets and variables → Actions):
   - `CLOUDFLARE_API_TOKEN` — create at dash.cloudflare.com/profile/api-tokens
     with the "Cloudflare Pages — Edit" template.
   - `CLOUDFLARE_ACCOUNT_ID` — visible on any zone's overview page.

After that, pushing changes to `site/` on `main` redeploys automatically;
`workflow_dispatch` allows manual redeploys from the Actions tab.

## Release coupling

The download buttons resolve the latest release at page-load time via the
GitHub API (`releases/latest`), so the site does **not** need a redeploy when
a new version ships — publishing the GitHub release is enough. Without JS (or
if the API is rate-limited) every button falls back to the GitHub releases
page.

## Manual deploy

```sh
cd site && npm ci && npm run build
npx wrangler pages deploy site/dist --project-name=boinc-portal
```
