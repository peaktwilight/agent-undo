#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_ROOT="${AGENT_UNDO_SMOKE_INSTALL_ROOT:-${RUNNER_TEMP:-${TMPDIR:-/tmp}}/agent-undo-smoke-root}"
WORK_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/agent-undo-smoke.XXXXXX")"
PROJECT_ROOT="${WORK_ROOT}/project"
FAKEBIN="${WORK_ROOT}/fakebin"

cleanup() {
  if [ -d "${PROJECT_ROOT}" ]; then
    (
      cd "${PROJECT_ROOT}" 2>/dev/null || exit 0
      "${INSTALL_ROOT}/bin/au" stop >/dev/null 2>&1 || true
    )
  fi
  rm -rf "${WORK_ROOT}"
}
trap cleanup EXIT

wait_until_contains() {
  local description="$1"
  local needle="$2"
  shift 2
  local output
  for _ in $(seq 1 50); do
    output="$("$@" 2>&1 || true)"
    if printf '%s' "${output}" | grep -Fq "${needle}"; then
      return 0
    fi
    sleep 0.2
  done
  printf 'timed out waiting for %s\nlast output:\n%s\n' "${description}" "${output}" >&2
  return 1
}

mkdir -p "${PROJECT_ROOT}" "${FAKEBIN}"

cargo install --path "${ROOT_DIR}" --root "${INSTALL_ROOT}" --force
export PATH="${INSTALL_ROOT}/bin:${PATH}"

au --version

cd "${PROJECT_ROOT}"
printf 'before\n' > story.txt

au init
au serve --daemon
wait_until_contains "daemon readiness" '"daemon_running": true' au status --json

au exec --agent smoke -- sh -c 'printf "after\n" > story.txt; sleep 1'
test "$(cat story.txt)" = "after"
wait_until_contains "smoke session" "smoke" au sessions
wait_until_contains "story modification in log" "modify story.txt" au log -n 20

printf '#!/usr/bin/env sh\nprintf "codex downstream\\n"\nprintf "wrapped-by-codex\\n" > codex-artifact.txt\nsleep 1\n' > "${FAKEBIN}/codex"
chmod +x "${FAKEBIN}/codex"

oops_output="$(au oops --confirm)"
printf '%s\n' "${oops_output}"
printf '%s' "${oops_output}" | grep -Fq 'story.txt'
test "$(cat story.txt)" = "before"

au wrap install --preset codex
export PATH="${FAKEBIN}:${PATH}"
eval "$(au wrap shellenv)"
wrapper_stdout="$(codex run smoke-check)"
test "${wrapper_stdout}" = "codex downstream"
test "$(cat codex-artifact.txt)" = "wrapped-by-codex"
wait_until_contains "codex session" "codex" au sessions
wait_until_contains "codex artifact in log" "codex-artifact.txt" au log -n 20

au stop
wait_until_contains "daemon shutdown" '"daemon_running": false' au status --json

printf 'smoke ok: install, daemon, exec attribution, wrapper path, and rollback verified\n'
