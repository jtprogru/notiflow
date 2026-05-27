#!/usr/bin/env bats

load helpers

setup() {
  setup_clean_env
  # shellcheck source=../scripts/lib.sh
  source "${NF_ROOT}/scripts/lib.sh"
  # shellcheck source=../scripts/filter.sh
  source "${NF_ROOT}/scripts/filter.sh"
}

@test "filter: status in list returns 0" {
  run bash -c 'source "'"${NF_ROOT}"'/scripts/lib.sh"; source "'"${NF_ROOT}"'/scripts/filter.sh"; nf::should_notify failure "success,failure"'
  [ "$status" -eq 0 ]
}

@test "filter: status not in list returns 1" {
  run bash -c 'source "'"${NF_ROOT}"'/scripts/lib.sh"; source "'"${NF_ROOT}"'/scripts/filter.sh"; nf::should_notify success "failure"'
  [ "$status" -eq 1 ]
}

@test "filter: whitespace tolerated" {
  run bash -c 'source "'"${NF_ROOT}"'/scripts/lib.sh"; source "'"${NF_ROOT}"'/scripts/filter.sh"; nf::should_notify success " success , failure "'
  [ "$status" -eq 0 ]
}

# CP-3: For all status × notify_on subsets, should_notify == (status ∈ notify_on).
# Note: bats `run` overrides $status — use a different variable name for the loop.
@test "filter: exclusion property (CP-3)" {
  local st lst expected actual
  for st in success failure cancelled skipped; do
    for lst in success failure cancelled skipped \
      "success,failure" \
      "failure,cancelled,skipped" \
      "success,failure,cancelled,skipped"; do
      case ",$lst," in
        *",$st,"*) expected=0 ;;
        *) expected=1 ;;
      esac
      bash -c '
        source "'"${NF_ROOT}"'/scripts/lib.sh"
        source "'"${NF_ROOT}"'/scripts/filter.sh"
        nf::should_notify "'"$st"'" "'"$lst"'"
      ' && actual=0 || actual=$?
      if [ "$actual" -ne "$expected" ]; then
        printf 'st=%s lst=%s expected=%s got=%s\n' "$st" "$lst" "$expected" "$actual" >&2
        return 1
      fi
    done
  done
}
