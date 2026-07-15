# GitHub self-hosted runner (VPS)

Deploy a GitHub Actions runner on the Boinc VPS (`bart@212.47.77.32`) so
workflows can use `runs-on: [self-hosted, linux, x64, vps]`.

Two install modes ship here:

| Mode | Path | When to use |
|------|------|-------------|
| **Host (recommended)** | `install-host.sh` + systemd | Building Boinc (GTK, deb/rpm, cargo). Full host tools. |
| **K3s pod (optional)** | `k8s/` + `deploy-k3s.sh` | Lightweight / isolated agent. Poor fit for full CI matrix. |

**Recommendation:** use the **host** install for this VPS. The cluster path is
documented for completeness but cannot compile `boinc-app` without a custom
image that has all native deps.

---

## Prerequisites

- VPS access: `ssh bart@212.47.77.32` (override with `VPS_USER` / `VPS_IP`)
- GitHub **admin** on `bartbeecoders/boinc` (to create runners / tokens)
- On the VPS: `curl`, `tar`, `systemd`, outbound HTTPS to `github.com` and
  `*.actions.githubusercontent.com`

Optional build tools if you want this runner to execute Boinc CI jobs later:

```bash
# Debian/Ubuntu on the VPS
sudo apt-get update
sudo apt-get install -y build-essential pkg-config \
  libgtk-3-dev libxkbcommon-dev libxdo-dev \
  curl ca-certificates git jq
# Rust (rustup) as the runner user
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

---

## 1. Host install (recommended)

### 1.1 Get a registration credential

**Option A — UI token (simplest, expires in ~1 hour)**

1. Open https://github.com/bartbeecoders/boinc/settings/actions/runners/new
2. Choose **Linux** / **x64**
3. Copy the token from the configure line (`--token AAAA…`)

**Option B — PAT (script mints a short-lived registration token)**

1. GitHub → Settings → Developer settings → Personal access tokens  
2. Fine-grained token on `bartbeecoders/boinc` with **Administration: Read and write**  
   (or classic PAT with `repo` for private repos; for public repos you still need admin)
3. Put it in `config.env` as `GITHUB_TOKEN` and **remove it after install**

### 1.2 Configure

On your laptop (or on the VPS):

```bash
cd runner
cp config.env.example config.env
chmod 600 config.env
# edit config.env — set REGISTRATION_TOKEN=... or GITHUB_TOKEN=...
```

Defaults assume:

- repo `bartbeecoders/boinc`
- runner name `vps-boinc-1`
- labels `vps,boinc` (plus automatic `self-hosted`, `Linux`, `X64`)
- install dir `/home/bart/actions-runner`
- user `bart`

### 1.3 Install on the VPS

```bash
# From your laptop: copy the runner folder
scp -r runner bart@212.47.77.32:~/boinc-runner

ssh bart@212.47.77.32
cd ~/boinc-runner
# ensure config.env is present (scp it separately if you keep secrets local)
./install-host.sh
```

What the script does:

1. Downloads the latest [actions/runner](https://github.com/actions/runner) release for the host arch  
2. Runs `sudo bin/installdependencies.sh` (libicu / .NET runtime deps — required on minimal Debian)  
3. Runs `config.sh --unattended` against the repo  
4. Installs and starts a **systemd** service via official `svc.sh`  
5. Enables linger so the agent survives reboot without an interactive login  

#### If you already hit the libicu / “svc.sh: command not found” error

`config.sh` aborted before it finished, so `svc.sh` was never produced and the
systemd unit was never installed. Recover on the VPS:

```bash
# 1. Install OS deps for the .NET runtime the agent uses
sudo ~/actions-runner/bin/installdependencies.sh

# 2. Ensure config.env still has a *fresh* REGISTRATION_TOKEN
#    (UI tokens expire ~1h — mint a new one if needed)
cd ~/boinc-runner
# refresh install-host.sh if you updated the repo copy
./install-host.sh
```

Or finish manually after dependencies are installed:

```bash
cd ~/actions-runner
./config.sh --unattended \
  --url https://github.com/bartbeecoders/boinc \
  --token YOUR_FRESH_TOKEN \
  --name vps-boinc-1 \
  --labels vps,boinc \
  --replace
sudo ./svc.sh install bart
sudo ./svc.sh start
sudo ./svc.sh status
```

### 1.4 Verify

```bash
# On the VPS
sudo ~/actions-runner/svc.sh status
journalctl -u 'actions.runner.*' -f
```

In GitHub:  
https://github.com/bartbeecoders/boinc/settings/actions/runners  
→ runner `vps-boinc-1` should show **Idle** (green).

### 1.5 Uninstall

```bash
# On the VPS (needs a fresh REGISTRATION_TOKEN or GITHUB_TOKEN to unregister cleanly)
cd ~/boinc-runner
./uninstall-host.sh
```

---

## 2. K3s install (optional)

Runs one pod in namespace `gha-runner` using
[`myoung34/github-runner`](https://github.com/myoung34/docker-github-actions-runner).

### 2.1 Secret

```bash
# PAT with Administration: Write on the repo
ssh bart@212.47.77.32
sudo k3s kubectl create namespace gha-runner --dry-run=client -o yaml | sudo k3s kubectl apply -f -
sudo k3s kubectl -n gha-runner create secret generic gha-runner \
  --from-literal=github_token='ghp_...' \
  --dry-run=client -o yaml | sudo k3s kubectl apply -f -
```

Or from the laptop after filling `runner/k8s/secret.yaml` (gitignored):

```bash
./runner/deploy-k3s.sh secret   # prints help
# create secret.yaml from secret.example.yaml, then:
./runner/deploy-k3s.sh apply
```

### 2.2 Deploy / status / logs

```bash
./runner/deploy-k3s.sh apply
./runner/deploy-k3s.sh status
./runner/deploy-k3s.sh logs
./runner/deploy-k3s.sh delete
```

Environment (edit `k8s/deployment.yaml` if needed):

| Variable | Default |
|----------|---------|
| `REPO_URL` | `https://github.com/bartbeecoders/boinc` |
| `LABELS` | `vps,boinc,k8s` |
| resources | request 256m/512Mi, limit 2 CPU / 4Gi |

---

## 3. Point a workflow at the runner

See [`workflow-example.yml`](workflow-example.yml). Minimal job:

```yaml
jobs:
  build:
    runs-on: [self-hosted, linux, x64, vps]
    steps:
      - uses: actions/checkout@v4
      # ...
```

Leave production CI on `ubuntu-latest` until the host has the same packages as
`.github/workflows/ci.yml` (GTK, etc.). A safe first step is a
`workflow_dispatch` smoke job only.

### Labels

| Label | Source |
|-------|--------|
| `self-hosted` | automatic |
| `Linux` / `X64` | automatic (host arch) |
| `vps`, `boinc` | `RUNNER_LABELS` in `config.env` / host install |
| `k8s` | only the in-cluster deployment |

---

## 4. Security notes

- **Self-hosted ≠ ephemeral VM.** A compromised workflow can leave persistence
  on the VPS. Do **not** run untrusted fork PR workflows on this runner without
  hardening (`pull_request_target` rules, label gates, etc.).
- **Secrets:** any job that runs on the runner receives repository secrets for
  that workflow. Treat the VPS as a production secret host.
- **Tokens:** registration tokens expire in about an hour. PATs in `config.env`
  or K8s secrets should be least-privilege and rotated; delete them from
  `config.env` after `install-host.sh` finishes.
- **User:** the default service user is `bart`. For stronger isolation create
  a dedicated `github-runner` user with no sudo and set `RUNNER_USER` /
  `RUNNER_HOME` accordingly.
- **Updates:** re-run `install-host.sh` to pull a newer `actions/runner`
  release (uses `--replace` for the same `RUNNER_NAME`).

---

## 5. Operations cheat sheet

```bash
# Status
sudo /home/bart/actions-runner/svc.sh status

# Restart
sudo /home/bart/actions-runner/svc.sh stop
sudo /home/bart/actions-runner/svc.sh start

# Logs
journalctl -u 'actions.runner.*' -n 100 --no-pager
journalctl -u 'actions.runner.*' -f

# Disk (work dirs can grow)
du -sh /home/bart/actions-runner/_work
```

### Firewall / network

The runner makes **outbound** HTTPS only (no inbound ports). Ensure the VPS
can reach:

- `github.com`
- `api.github.com`
- `*.actions.githubusercontent.com`
- `codeload.github.com`
- `objects.githubusercontent.com`

---

## 6. Layout

```
runner/
  README.md              ← this file
  config.env.example     ← copy to config.env (gitignored)
  install-host.sh        ← recommended install (run on VPS)
  uninstall-host.sh
  deploy-k3s.sh          ← optional in-cluster deploy from laptop
  workflow-example.yml   ← sample workflow using the runner
  .gitignore
  k8s/
    namespace.yaml
    deployment.yaml
    secret.example.yaml
```

## 7. VPS defaults (this project)

| Setting | Value |
|---------|--------|
| Host | `212.47.77.32` |
| SSH | `bart@212.47.77.32` |
| Repo | `bartbeecoders/boinc` |
| Suggested runner name | `vps-boinc-1` |
| Suggested labels | `vps,boinc` |
