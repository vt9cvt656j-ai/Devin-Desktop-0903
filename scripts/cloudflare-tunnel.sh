#!/usr/bin/env bash
#
# Expose the local Devin Desktop bridge to the internet through a Cloudflare
# Tunnel, so a cloud agent (Devin) can reach a machine that has no public
# inbound IP — e.g. a physical box behind NAT / a carrier-grade firewall in
# mainland China. The tunnel is an *outbound* connection from this machine to
# Cloudflare's edge, so no port-forwarding or public IP is required.
#
# Two modes:
#
#   quick   Throwaway https://<random>.trycloudflare.com URL. Zero config, but
#           the hostname changes every run and the edge can be flaky from some
#           networks. Good for a one-off test.
#
#   named   A stable tunnel bound to your own Cloudflare-managed domain. This is
#           the recommended setup for anything you use more than once. Requires
#           a Cloudflare account with a zone (domain) added.
#
# Usage:
#   scripts/cloudflare-tunnel.sh quick [PORT]
#   scripts/cloudflare-tunnel.sh named HOSTNAME [PORT] [TUNNEL_NAME]
#
# Examples:
#   scripts/cloudflare-tunnel.sh quick 53412
#   scripts/cloudflare-tunnel.sh named devin.example.com 53412 devin-bridge
#
# PORT defaults to the value of $BRIDGE_PORT, otherwise 53412. Copy the exact
# port (and token) from the Devin Desktop app window.

set -euo pipefail

DEFAULT_PORT="${BRIDGE_PORT:-53412}"

die() { echo "error: $*" >&2; exit 1; }

require_cloudflared() {
  if ! command -v cloudflared >/dev/null 2>&1; then
    cat >&2 <<'EOF'
error: `cloudflared` is not installed.

Install it first:
  macOS:          brew install cloudflared
  Debian/Ubuntu:  https://pkg.cloudflare.com/  (apt repo) or download the .deb
  Other Linux:    download the binary from
                  https://github.com/cloudflare/cloudflared/releases

EOF
    exit 1
  fi
}

usage() {
  sed -n '2,28p' "$0" | sed 's/^# \{0,1\}//'
  exit "${1:-0}"
}

cmd="${1:-}"
[ -n "$cmd" ] || usage 1

case "$cmd" in
  quick)
    require_cloudflared
    port="${2:-$DEFAULT_PORT}"
    url="http://127.0.0.1:${port}"
    echo "Starting a quick Cloudflare Tunnel to ${url}"
    echo "Watch the output for the https://<...>.trycloudflare.com URL, then give"
    echo "that URL + your bridge token to Devin."
    echo
    exec cloudflared tunnel --url "$url"
    ;;

  named)
    require_cloudflared
    hostname="${2:-}"
    [ -n "$hostname" ] || die "named mode requires HOSTNAME, e.g. devin.example.com"
    port="${3:-$DEFAULT_PORT}"
    tunnel_name="${4:-devin-bridge}"
    url="http://127.0.0.1:${port}"

    # One-time browser login that authorizes this machine for your account/zone.
    if [ ! -f "${HOME}/.cloudflared/cert.pem" ]; then
      echo "No Cloudflare cert found — launching login (a browser window opens)..."
      cloudflared tunnel login
    fi

    # Create the tunnel if it does not already exist.
    if ! cloudflared tunnel list 2>/dev/null | awk '{print $2}' | grep -qx "$tunnel_name"; then
      echo "Creating tunnel '${tunnel_name}'..."
      cloudflared tunnel create "$tunnel_name"
    else
      echo "Reusing existing tunnel '${tunnel_name}'."
    fi

    # Point the DNS record at the tunnel (idempotent; ignore 'already exists').
    echo "Routing ${hostname} -> ${tunnel_name}..."
    cloudflared tunnel route dns "$tunnel_name" "$hostname" || true

    echo
    echo "Starting tunnel. Public URL: https://${hostname}"
    echo "Give Devin:  https://${hostname}  + your bridge token."
    echo
    exec cloudflared tunnel run --url "$url" "$tunnel_name"
    ;;

  -h|--help|help)
    usage 0
    ;;

  *)
    die "unknown command '$cmd' (expected 'quick' or 'named'); run with --help"
    ;;
esac
