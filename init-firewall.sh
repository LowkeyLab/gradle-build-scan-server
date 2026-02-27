#!/usr/bin/env bash
#
# Initialise an allowlist-only outbound firewall.
# Run with: sudo /usr/local/bin/init-firewall.sh
#
# Allowed destinations:
#   - DNS (UDP 53)
#   - SSH (TCP 22)
#   - Localhost / Docker host network
#   - api.anthropic.com
#   - GitHub (fetched from api.github.com/meta)
#   - npm registry (registry.npmjs.org)
#   - Sentry & Statsig telemetry
#
set -euo pipefail

# ── Create ipset for allowed IPs ────────────────────────────────────
ipset create allowed_ips hash:net -exist
ipset flush allowed_ips

# Localhost
ipset add allowed_ips 127.0.0.0/8
ipset add allowed_ips ::1/128

# Docker default bridge
ipset add allowed_ips 172.17.0.0/16

# Resolve and add IPs for a given hostname
add_host() {
  local host="$1"
  for ip in $(dig +short "$host" A 2>/dev/null || true); do
    [[ "$ip" =~ ^[0-9]+\. ]] && ipset add allowed_ips "$ip/32" -exist
  done
  for ip in $(dig +short "$host" AAAA 2>/dev/null || true); do
    [[ "$ip" =~ : ]] && ipset add allowed_ips "$ip/128" -exist
  done
}

# Anthropic API
add_host api.anthropic.com

# npm registry
add_host registry.npmjs.org

# Sentry & Statsig (telemetry)
add_host sentry.io
add_host statsig.anthropic.com
add_host statsig.com

# GitHub IPs (from their meta endpoint)
if command -v curl &>/dev/null && command -v jq &>/dev/null; then
  gh_meta=$(curl -fsSL https://api.github.com/meta 2>/dev/null || echo '{}')
  for cidr in $(echo "$gh_meta" | jq -r '(.web // [])[] , (.api // [])[] , (.git // [])[] , (.actions // [])[]' 2>/dev/null | sort -u); do
    ipset add allowed_ips "$cidr" -exist 2>/dev/null || true
  done
fi

# ── iptables rules ──────────────────────────────────────────────────

# Allow all loopback
iptables  -A OUTPUT -o lo -j ACCEPT
ip6tables -A OUTPUT -o lo -j ACCEPT

# Allow established/related (so replies to our allowed requests come back)
iptables  -A OUTPUT -m state --state ESTABLISHED,RELATED -j ACCEPT
ip6tables -A OUTPUT -m state --state ESTABLISHED,RELATED -j ACCEPT

# Allow DNS
iptables  -A OUTPUT -p udp --dport 53 -j ACCEPT
ip6tables -A OUTPUT -p udp --dport 53 -j ACCEPT

# Allow SSH
iptables  -A OUTPUT -p tcp --dport 22 -j ACCEPT
ip6tables -A OUTPUT -p tcp --dport 22 -j ACCEPT

# Allow traffic to our allowlisted IPs (HTTP/HTTPS)
iptables  -A OUTPUT -p tcp --dport 80  -m set --match-set allowed_ips dst -j ACCEPT
iptables  -A OUTPUT -p tcp --dport 443 -m set --match-set allowed_ips dst -j ACCEPT
ip6tables -A OUTPUT -p tcp --dport 80  -m set --match-set allowed_ips dst -j ACCEPT
ip6tables -A OUTPUT -p tcp --dport 443 -m set --match-set allowed_ips dst -j ACCEPT

# Drop everything else outbound
iptables  -A OUTPUT -j DROP
ip6tables -A OUTPUT -j DROP

echo "Firewall initialised — outbound traffic restricted to allowlist."
