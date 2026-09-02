# Exposing the bridge with Cloudflare Tunnel

Devin runs in the cloud and cannot reach your machine's `localhost` directly. If
your machine also has **no public inbound IP** — e.g. a physical box behind NAT,
behind a corporate firewall, or on a network that blocks foreign inbound traffic
(common in mainland China) — you cannot "allow the cloud IP in". The fix is a
**reverse, outbound tunnel**: your machine dials *out* to Cloudflare's edge, and
Devin connects to the public URL that the edge exposes.

```
  Devin (cloud)  ──HTTPS──▶  Cloudflare edge  ◀──outbound tunnel──  your machine
                                                                    127.0.0.1:<port>
```

No port-forwarding, no public IP, and nothing inbound to open on your firewall.

The bridge still enforces its **bearer token** on every request, so the public
URL is useless without the token. Keep the token secret and prefer **read-only**
mode when you can.

## Prerequisites

- The Devin Desktop bridge running. Copy the **port** and **token** from the app
  window (the local URL looks like `http://127.0.0.1:53412`).
- `cloudflared` installed:
  - macOS: `brew install cloudflared`
  - Debian/Ubuntu: add the [Cloudflare apt repo](https://pkg.cloudflare.com/) or
    download the `.deb`
  - Other: grab a binary from the
    [releases page](https://github.com/cloudflare/cloudflared/releases)

A helper script wraps both modes below:
[`scripts/cloudflare-tunnel.sh`](../scripts/cloudflare-tunnel.sh).

## Option A — quick tunnel (one-off test)

Fastest path, zero account needed. The hostname is random and changes every run.

```bash
scripts/cloudflare-tunnel.sh quick 53412
# or directly:
cloudflared tunnel --url http://127.0.0.1:53412
```

`cloudflared` prints a `https://<random>.trycloudflare.com` URL. Give that URL +
your token to Devin.

> Note for mainland China: `trycloudflare.com` and the nearest Cloudflare edge
> can be slow or intermittently unreachable. If the quick tunnel is flaky, use a
> named tunnel on your own domain (Option B), which is far more stable.

## Option B — named tunnel on your own domain (recommended)

A stable `https://devin.example.com` that survives restarts. Requires a free
Cloudflare account with a domain (zone) added to it.

```bash
scripts/cloudflare-tunnel.sh named devin.example.com 53412 devin-bridge
```

What the script does (all idempotent, safe to re-run):

1. `cloudflared tunnel login` — one-time browser auth for your account/zone
   (only runs if `~/.cloudflared/cert.pem` is missing).
2. `cloudflared tunnel create devin-bridge` — creates the tunnel if absent.
3. `cloudflared tunnel route dns devin-bridge devin.example.com` — points the
   hostname at the tunnel.
4. `cloudflared tunnel run --url http://127.0.0.1:53412 devin-bridge` — starts it.

Then give Devin `https://devin.example.com` + your token.

### Run it permanently (systemd, Linux)

For a physical box you want always-on, install `cloudflared` as a service so the
tunnel comes back after reboots. After completing the `login`/`create`/`route`
steps above once:

```bash
# Tell the service which tunnel + local target to run.
sudo mkdir -p /etc/cloudflared
sudo tee /etc/cloudflared/config.yml >/dev/null <<'YAML'
tunnel: devin-bridge
credentials-file: /root/.cloudflared/<TUNNEL_UUID>.json
ingress:
  - hostname: devin.example.com
    service: http://127.0.0.1:53412
  - service: http_status:404
YAML

sudo cloudflared service install
sudo systemctl enable --now cloudflared
sudo systemctl status cloudflared
```

Replace `<TUNNEL_UUID>.json` with the credentials file that `tunnel create`
printed (find it under `~/.cloudflared/`). If you started the bridge on a
different port, change `53412` to match.

## Security checklist

- Keep the **bearer token** secret; rotate it by restarting the bridge.
- The public URL is HTTPS end-to-end (Cloudflare edge ⇄ your machine over the
  tunnel), so traffic is encrypted.
- Use the app's **read-only** switch unless Devin genuinely needs to write.
- The bridge is confined to the **single folder** you picked; it can never read
  or write outside it.
