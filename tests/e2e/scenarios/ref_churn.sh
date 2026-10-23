ref_churn_suite() {
    local host_client="$1" label churn_pid churn_exe="$fixture_app/Contents/MacOS/AgentDeskFixture"
    local filter alpha stable output ok code delivery beta_seen stable_write stable_seen
    local -x AGENT_DESKTOP_INTERNAL_HOST_CLIENT="$host_client"
    local -x TMPDIR="$host_tmp"
    label="[ref-churn host_client=$host_client]"
    MODE_FLAG=""
    note "$label strict re-identification after the collection changes"
    if [ -n "$(pgrep -x "$app" 2>/dev/null || true)" ]; then
        badmsg "$label another fixture process is already running"
        return 0
    fi
    AGENT_DESKTOP_FIXTURE_REF_CHURN=1 AGENT_DESKTOP_FIXTURE_NO_ACTIVATE=1 "$churn_exe" >/dev/null 2>&1 &
    churn_pid=$!
    fixture_owned=1
    for _ in $(seq 1 50); do
        find_target_by_id button filter-rows >/dev/null 2>&1 && break
        sleep 0.2
    done
    require_target_by_id filter button filter-rows
    require_target_by_id alpha textfield shared-title
    require_target_by_id stable textfield stable-editor
    act_target "$filter" click >/dev/null 2>&1
    "$bin" wait --text "Restore rows" --app "$app" --timeout 5000 >/dev/null 2>&1
    output="$("$bin" set-value "$(target_ref "$alpha")" poisoned 2>&1)"
    ok="$(json_field "$output" ok)"
    code="$(json_field "$output" error.code)"
    delivery="$(json_field "$output" error.disposition.delivery)"
    assert "$label stale ref is refused STALE_REF, not delivered" \
        "$([ "$ok" = "False" ] && [ "$code" = "STALE_REF" ] && [ "$delivery" = "not_delivered" ] && echo 1 || echo 0)" \
        "ok=$ok code=$code delivery=$delivery"
    beta_seen="$("$bin" wait --text "Beta stored: Beta" --app "$app" --timeout 3000 2>&1)"
    assert "$label sibling kept its value (independent readout)" \
        "$([ "$(json_field "$beta_seen" data.found)" = "True" ] && echo 1 || echo 0)" \
        "found=$(json_field "$beta_seen" data.found)"
    stable_write="$("$bin" set-value "$(target_ref "$stable")" stable-ok 2>&1)"
    stable_seen="$(wait_target "$stable" value 3000 --value stable-ok 2>&1)"
    assert "$label an untouched ref still resolves and writes" \
        "$([ "$(json_field "$stable_write" ok)" = "True" ] && [ "$(json_field "$stable_seen" data.found)" = "True" ] && echo 1 || echo 0)" \
        "ok=$(json_field "$stable_write" ok) found=$(json_field "$stable_seen" data.found)"
    "$bin" close-app "$app" --force >/dev/null 2>&1
    for _ in $(seq 1 25); do
        [ -z "$(pgrep -x "$app" 2>/dev/null || true)" ] && break
        sleep 0.2
    done
    if [ -n "$(pgrep -x "$app" 2>/dev/null || true)" ]; then
        badmsg "$label fixture still running after close-app"
        kill "$churn_pid" 2>/dev/null || true
    fi
    wait "$churn_pid" 2>/dev/null
    fixture_owned=0
}

ref_churn_suite 0
ref_churn_suite 1
