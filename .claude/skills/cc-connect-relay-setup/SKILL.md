---
name: cc-connect-relay-setup
description: Stand up a self-hosted iroh-relay on a user-supplied Linux server so cc-connect can route gossip + iroh-blobs through their own infrastructure instead of n0's public relays. Walks through SSH access, TLS cert issuance via certbot, nginx reverse-proxy, iroh-relay binary install, and a systemd unit. Use when the user asks to "self-host the relay", "set up my own relay", "use my own server for cc-connect", or anything in that intent.
---

# Self-hosted iroh-relay setup

Goal: take a Linux server the user owns + a (sub)domain they control, and stand up a working iroh-relay that cc-connect's `--relay` flag can target. End state: `https://relay.<their-domain>` returns the "Iroh Relay" landing page, a `cc-connect` chat using `--relay https://relay.<their-domain>` connects without ever talking to n0.

## Inputs to collect (ask via AskUserQuestion or plain question)

Ask in this order, one at a time. Don't dump a wall of questions.

1. **Server SSH target** — `user@host` (e.g. `yijian@124.243.176.37`). Verify the user has key-based SSH set up (`ssh -o BatchMode=yes <target> 'echo ok'` should succeed). If it asks for a password, stop and ask the user to add their key to the server's `~/.ssh/authorized_keys`. Don't attempt password auth — it leaks via process args and we can't pipe interactive prompts safely.
2. **Domain for the relay** — the full hostname (e.g. `relay.example.com`). Verify it resolves to the server's IP via `dig +short <hostname> A` from your local machine. If it doesn't, stop and ask the user to add an A record first; don't proceed with cert issuance until DNS is right.
3. **Email for Let's Encrypt** — required for the renewal-warning notifications. Falls into the certbot `--email` flag.

## Server prerequisites you check

Before doing anything:

- `ssh <target> 'sudo -n true 2>&1'` — must print nothing (passwordless sudo) or you can't proceed without prompting per-step. If it prompts, ask the user if they want to enter their sudo password each step or grant NOPASSWD temporarily.
- `ssh <target> 'which nginx certbot'` — both required. If missing: `sudo apt-get install -y nginx certbot python3-certbot-nginx`.
- `ssh <target> 'ss -tln | grep -E ":(80|443)\s"'` — confirm 80 and 443 are bound by nginx (or free). If something else owns them (apache, custom server), STOP and surface what's listening; the user has to decide whether to coexist.
- `ssh <target> 'which cargo rustc'` — if missing, install via `curl --proto =https --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal` then `source $HOME/.cargo/env`.

## Build steps (run remotely)

Each step: explain what you're about to do, then run, then verify before moving on.

### 1. Install the binary

```bash
ssh <target> 'source $HOME/.cargo/env && cargo install iroh-relay --version 0.97.0 --features server --locked'
```

This takes 3-5 minutes (heavy iroh dependency tree). Run as a background task and monitor progress. If install fails, capture the last 30 lines of cargo output and surface them; common failures are `rustc` too old (need ≥ 1.85) or out-of-memory on tiny VMs (< 1 GiB RAM — give it swap or a bigger box).

### 2. Drop the iroh-relay TOML config

Bind to `127.0.0.1:8443` (plain HTTP — nginx terminates TLS in step 4). Disable QUIC address discovery (it needs UDP that nginx can't proxy; we trade NAT-traversal hint for nginx co-existence).

```bash
ssh <target> 'sudo install -d -m 755 /etc/iroh-relay && sudo tee /etc/iroh-relay/config.toml > /dev/null' << 'EOF'
enable_relay = true
http_bind_addr = "127.0.0.1:8443"
enable_quic_addr_discovery = false
enable_metrics = true
metrics_bind_addr = "127.0.0.1:9090"
EOF
```

### 3. systemd unit

```bash
ssh <target> 'sudo tee /etc/systemd/system/iroh-relay.service > /dev/null' << 'EOF'
[Unit]
Description=iroh-relay (cc-connect)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=<ssh-user>
ExecStart=/home/<ssh-user>/.cargo/bin/iroh-relay --config-path /etc/iroh-relay/config.toml
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF
```

Substitute `<ssh-user>` with the user's actual login name (the part before `@` in their SSH target).

### 4. nginx vhost (port 80, HTTP-only — certbot will add 443 SSL)

```bash
ssh <target> "sudo tee /etc/nginx/sites-available/iroh-relay.<domain>.conf > /dev/null << 'EOF'
server {
    listen 80;
    server_name <domain>;
    location / { return 301 https://\$host\$request_uri; }
}
server {
    listen 443 ssl;
    server_name <domain>;
    location / {
        proxy_pass http://127.0.0.1:8443;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection \"upgrade\";
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        proxy_buffering off;
        proxy_read_timeout 86400s;
    }
}
EOF
sudo ln -sf /etc/nginx/sites-available/iroh-relay.<domain>.conf /etc/nginx/sites-enabled/"
```

### 5. Issue the Let's Encrypt cert

```bash
ssh <target> 'sudo certbot --nginx --non-interactive --agree-tos --email <email> -d <domain>'
```

certbot edits the vhost in place and reloads nginx. If this fails with "DNS does not resolve to this server", go back to step 0 — the A record needs to point here.

### 6. Start + verify

```bash
ssh <target> 'sudo systemctl daemon-reload && sudo systemctl enable --now iroh-relay && sleep 2 && sudo systemctl status iroh-relay --no-pager | head -15'
```

Then verify from the user's local machine:

```bash
curl -sk https://<domain>/ -w "\nHTTP %{http_code}\n"
```

Expect `HTTP 200` with HTML body containing "Iroh Relay". If you get `502`, the systemd unit is down — `journalctl -u iroh-relay -n 30` on the server. If you get `404`, nginx is reaching the relay but the path is wrong — usually fine, the iroh client doesn't hit `/`.

### 7. Hand off

Tell the user the relay is live at `https://<domain>` and how to use it:

- Host a room through the new relay: `cc-connect host --relay https://<domain>`
- Joiners using the printed ticket inherit the relay automatically; they don't need `--relay` themselves unless they want to override.
- Logs: `sudo journalctl -u iroh-relay -f`. Cert renewal is on certbot's timer; nothing to schedule.

## Things you should NOT do

- **Don't proxy UDP through nginx** — it can't (nginx is HTTP/TCP). If the user wants QUIC address discovery for hole-punching they need a separate UDP socket on the relay (3478 by default), bound directly to the public IP without going through nginx. Flag this as a v2 polish, not part of this skill.
- **Don't issue a wildcard cert** unless the user explicitly asks. Single-domain HTTP-01 is simpler and cleaner.
- **Don't run iroh-relay as root.** Always create a dedicated systemd `User=` (the SSH user is fine for hobby boxes; for prod, suggest `useradd --system --no-create-home iroh-relay`).
- **Don't skip the `dig` DNS check.** certbot's failure mode is ugly when DNS is wrong; do the cheap pre-check.
