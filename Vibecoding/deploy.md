# Deploying the Boinc portal (boinc.hideterms.com)

The portal is a React + Vite app in `site/` (`npm run dev` to work on it).
It is served from the VPS K3S cluster as an nginx container, exposed on
**NodePort 32087**; a Cloudflare Tunnel (configured outside this repo)
forwards `https://boinc.hideterms.com` to `<node>:32087`.

## Deploy

```sh
./scripts/deploy-k3s.sh          # build → push → deploy → status
```

Steps it performs:
1. `podman build` the image from `site/Dockerfile` (node builder → nginx),
   tagged `beecodersregistry.azurecr.io/boinc-site:{latest,<version>}` with
   the version from `site/package.json`.
2. Push to the ACR registry (assumes `podman login beecodersregistry.azurecr.io`
   has been run; or set `REGISTRY_USER`/`REGISTRY_PASSWORD`).
3. Copy `k8s/boinc/` manifests to the VPS (`bart@212.47.77.32`, override with
   `VPS_IP`/`VPS_USER`), apply namespace → acr-secret (copied from the
   aidbooks/sqail namespace if missing) → service → deployment, then force a
   rollout restart so the new `:latest` digest is pulled.

Subcommands: `build`, `push`, `deploy`, `status`, `logs`.

## Health

```sh
curl http://212.47.77.32:32087/healthz   # "ok" from the nginx pod
```

## Cloudflare Tunnel (manual, one-time)

Add a public hostname to the existing tunnel on the VPS:
`boinc.hideterms.com` → `http://localhost:32087`. DNS for the hostname is
created by the tunnel config; no other DNS records are needed.

## Release coupling

The download buttons resolve the latest release at page-load time via the
GitHub API (`releases/latest`), so the site does **not** need a redeploy when
a new version ships — publishing the GitHub release is enough. Without JS (or
if the API is rate-limited) every button falls back to the GitHub releases
page.
