#!/usr/bin/env bash
# test-all.sh - Run full test suite against SQLite and geometric backends.
#
# This script tests splice with supported backend configurations to ensure
# compatibility and correctness across all supported backends.
#
# Usage: ./scripts/test-all.sh [--quick] [--backend sqlite|geometric]
#   --quick: Skip long-running tests (cargo test --skip)
#   --backend: Test only the specified backend

set -euo pipefail

# Color output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly NC='\033[0m' # No Color

# Test configuration
BACKEND_SQLITE="sqlite"
BACKEND_GEOMETRIC="geometric"
QUICK_MODE=false
SELECTED_BACKEND=""

# Parse arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --quick)
      QUICK_MODE=true
      shift
      ;;
    --backend)
      SELECTED_BACKEND="$2"
      shift 2
      ;;
    -h|--help)
      echo "Usage: $0 [--quick] [--backend sqlite|geometric]"
      echo ""
      echo "Options:"
      echo "  --quick           Skip long-running tests"
      echo "  --backend BACKEND Test only sqlite or geometric backend"
      echo "  -h, --help        Show this help message"
      exit 0
      ;;
    *)
      echo -e "${RED}Unknown option: $1${NC}"
      exit 1
      ;;
  esac
done

# Print section header
header() {
  echo -e "\n${YELLOW}=== $1 ===${NC}\n"
}

# Test a single backend
test_backend() {
  local backend=$1
  local features=""

  if [[ "$backend" == "$BACKEND_SQLITE" ]]; then
    # SQLite is default, no features needed
    features=""
  elif [[ "$backend" == "$BACKEND_GEOMETRIC" ]]; then
    features="--features geometric"
  else
    echo -e "${RED}Unknown backend: $backend${NC}"
    return 1
  fi

  header "Testing $backend backend"

  local test_cmd="cargo test $features"

  if [[ "$QUICK_MODE" == true ]]; then
    test_cmd="$test_cmd -- --skip test_slow --skip test_performance"
  fi

  echo -e "${GREEN}Running: $test_cmd${NC}"

  if eval "$test_cmd"; then
    echo -e "${GREEN}✓ $backend backend tests PASSED${NC}"
    return 0
  else
    local exit_code=$?
    echo -e "${RED}✗ $backend backend tests FAILED (exit code: $exit_code)${NC}"
    return $exit_code
  fi
}

# Main test execution
main() {
  local backends_to_test=()
  local failed_backends=()

  if [[ -n "$SELECTED_BACKEND" ]]; then
    backends_to_test=("$SELECTED_BACKEND")
  else
    backends_to_test=("$BACKEND_SQLITE" "$BACKEND_GEOMETRIC")
  fi

  header "Splice Backend Test Suite"
  echo "Testing ${#backends_to_test[@]} backend(s): ${backends_to_test[*]}"
  [[ "$QUICK_MODE" == true ]] && echo "Quick mode enabled (skipping long-running tests)"

  for backend in "${backends_to_test[@]}"; do
    if ! test_backend "$backend"; then
      failed_backends+=("$backend")
    fi
  done

  # Summary
  header "Test Summary"

  if [[ ${#failed_backends[@]} -eq 0 ]]; then
    echo -e "${GREEN}All tests PASSED!${NC}"
    exit 0
  else
    echo -e "${RED}Failed backends: ${failed_backends[*]}${NC}"
    exit 1
  fi
}

main "$@"
