#!/usr/bin/env bash
# Developer diagnostics only. Never a product transport, native admission, grant,
# model qualification or permission to change host network/trust configuration.
# Caller MUST hold the shared heavy reservation and existing capped scope.
# Usage: bash scripts/research_transport_fixtures.sh EXACT_TEST_BINARY PRIVATE_STATE_DIR
set -euo pipefail
umask 077
research_script="$(realpath -- "${BASH_SOURCE[0]}")"
research_root="$(dirname -- "$(dirname -- "$research_script")")"
research_data="$research_root/fixtures/public-research/namespace"

if [[ "${1:-}" == --namespace ]]; then
  research_binary="${2:?test binary}"
  research_outer_net="${3:?outer network namespace}"
  research_outer_mnt="${4:?outer mount namespace}"
  research_case="${5:?case}"
  test "$(readlink /proc/self/ns/net)" != "$research_outer_net"
  test "$(readlink /proc/self/ns/mnt)" != "$research_outer_mnt"
  test "$(id -u)" = 0
  # Root only INSIDE a single-user namespace, never privileged host-root.
  awk '{ if (NR != 1 || $1 != 0 || $2 == 0 || $3 != 1) exit 1 } END { if (NR != 1) exit 1 }' /proc/self/uid_map
  /usr/bin/ip link set lo up
  /usr/bin/ip address add 93.184.216.34/32 dev lo
  # These mounts alter ONLY the disposable private mount namespace. The existing
  # host resolver, routes, certificates, services and listeners are untouched.
  # Also hide nscd/resolver sockets: a network namespace alone does not isolate
  # filesystem Unix sockets. Fresh /etc avoids following the host resolver symlink.
  /usr/bin/mount -t tmpfs -o nosuid,nodev,noexec,size=1m tmpfs /etc
  /usr/bin/mount -t tmpfs -o nosuid,nodev,noexec,size=1m tmpfs /run
  /usr/bin/install -m 0444 "$research_data/resolv.conf" /etc/resolv.conf
  /usr/bin/install -m 0444 "$research_data/nsswitch.conf" /etc/nsswitch.conf
  if [[ "$research_case" == dns ]]; then
    exec env -i LANG=C RUST_TEST_THREADS=1 \
      AGENTMAGE_TEST_OUTER_net="$research_outer_net" AGENTMAGE_TEST_OUTER_mnt="$research_outer_mnt" \
      HTTPS_PROXY=http://127.0.0.1:3128 \
      "$research_binary" transport::namespace_tests:: --ignored --nocapture --test-threads=1 \
      --skip real_tls_verifies_hostname_and_trust_then_enforces_response_and_redirect_bounds
  fi
  test "$research_case" = tls
  research_tls="${6:?fresh synthetic TLS directory}"
  cd -- "$research_tls"
  # This shell is PID1 in the disposable PID namespace: descendants cannot survive
  # its exit. Track, terminate and wait for only its one owned fixture process.
  OPENSSL_CONF=/dev/null /usr/bin/openssl s_server -accept 93.184.216.34:443 -cert leaf.pem -key leaf.key \
    -HTTP -no_cache -no_ticket > server.log 2>&1 &
  research_server_pid=$!
  cleanup_fixture() {
    research_status=$?
    trap - EXIT TERM INT
    if jobs -pr | rg -q "^${research_server_pid}$"; then kill -TERM "$research_server_pid"; fi
    wait "$research_server_pid" || true
    printf 'RESEARCH_TLS_FIXTURE_OWNED_SERVER_REAPED=1\n'
    exit "$research_status"
  }
  trap cleanup_fixture EXIT
  trap 'exit 130' TERM INT
  for research_attempt in {1..50}; do
    if rg -q '^ACCEPT' server.log; then break; fi
    if ! jobs -pr | rg -q "^${research_server_pid}$"; then
      printf 'Owned TLS server exited before readiness\n' >&2
      exit 1
    fi
    sleep 0.02
  done
  rg -q '^ACCEPT' server.log
  env -i LANG=C RUST_TEST_THREADS=1 \
    AGENTMAGE_TEST_OUTER_net="$research_outer_net" AGENTMAGE_TEST_OUTER_mnt="$research_outer_mnt" \
    AGENTMAGE_TEST_TLS_FIXTURE="$research_tls" HTTPS_PROXY=http://127.0.0.1:3128 \
    "$research_binary" transport::namespace_tests::real_tls_verifies_hostname_and_trust_then_enforces_response_and_redirect_bounds \
    --exact --ignored --nocapture --test-threads=1
  exit 0
fi

test "$#" = 2
test "${CARGO_BUILD_JOBS:-}" = 1
test "${RUST_TEST_THREADS:-}" = 1
awk '/MemAvailable:/ { found=1; if ($2 < 16777216) exit 1 } END { if (!found) exit 1 }' /proc/meminfo
research_binary="$(realpath -- "$1")"
research_state="$(realpath -- "$2")"
test -f "$research_binary" && test -x "$research_binary"
test -d "$research_state"
case "$research_binary" in "$research_root"/target/debug/deps/agentmage_public_research_worker-*) ;; *) exit 1 ;; esac
research_outer_net="$(readlink /proc/self/ns/net)"
research_outer_mnt="$(readlink /proc/self/ns/mnt)"
sha256sum "$research_binary" "$research_script" "$research_data"/* /usr/bin/unshare /usr/bin/ip /usr/bin/mount /usr/bin/openssl
timeout --foreground --kill-after=2s 20s unshare --user --map-root-user --net --mount \
  --propagation private --pid --mount-proc --fork --kill-child \
  bash "$research_script" --namespace "$research_binary" "$research_outer_net" "$research_outer_mnt" dns
research_tls="$(mktemp -d "$research_state/research-tls-XXXXXXXX")"
printf 'TLS_FIXTURE_DIRECTORY=%s\n' "$research_tls"
/usr/bin/openssl req -x509 -newkey rsa:2048 -noenc -days 1 -subj /CN=AgentMage-Synthetic-Fixture-CA \
  -addext basicConstraints=critical,CA:TRUE -addext keyUsage=critical,keyCertSign,cRLSign \
  -keyout "$research_tls/ca.key" -out "$research_tls/ca.pem" > "$research_tls/ca-generation.log" 2>&1
/usr/bin/openssl req -new -newkey rsa:2048 -noenc -subj /CN=docs.example.com \
  -keyout "$research_tls/leaf.key" -out "$research_tls/leaf.csr" > "$research_tls/leaf-generation.log" 2>&1
/usr/bin/openssl x509 -req -in "$research_tls/leaf.csr" -CA "$research_tls/ca.pem" -CAkey "$research_tls/ca.key" \
  -set_serial 1 -days 1 -extfile "$research_data/leaf.ext" -out "$research_tls/leaf.pem"
sha256sum "$research_tls/ca.pem" "$research_tls/leaf.pem"
timeout --foreground --kill-after=2s 20s unshare --user --map-root-user --net --mount \
  --propagation private --pid --mount-proc --fork --kill-child \
  bash "$research_script" --namespace "$research_binary" "$research_outer_net" "$research_outer_mnt" tls "$research_tls"
printf 'RESEARCH_TRANSPORT_FIXTURES_COMPLETE=1\n'
