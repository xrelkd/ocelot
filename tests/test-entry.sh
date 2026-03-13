#!/usr/bin/env bash

set -euo pipefail

if [[ -n ${1:-} ]]; then
    OCELOT_BIN="$1"
elif [[ -x "./target/release/ocelot" ]]; then
    OCELOT_BIN="./target/release/ocelot"
elif [[ -x "./target/debug/ocelot" ]]; then
    OCELOT_BIN="./target/debug/ocelot"
else
    echo "❌ Error: Could not find ocelot binary in target/release or target/debug."
    echo "Please run 'cargo build' or 'cargo build --release' first."
    exit 1
fi

echo "🚀 Testing with: $OCELOT_BIN"

echo "--- Running Integration Test: True PID 1 Mode ---"

unshare -rfp --mount-proc "$OCELOT_BIN" entry -- bash <<EOF
    echo "--- Inside Namespace ---"
    echo "Check current PID (should be 2, because ocelot is 1): \$\$"

    $OCELOT_BIN zombie -c 20 -i 50
    ZOMBIE_GEN_PID=\$!

    echo "--- Process Table (Checking for Reaping) ---"
    ps auxf

    Z_COUNT=\$(ps aux | grep 'defunct' | grep -v grep | wc -l || true)

    if [ "\$Z_COUNT" -eq "0" ]; then
        echo "✅ SUCCESS: PID 1 (ocelot) is actively reaping orphans."
    else
        echo "❌ FAILURE: Found \$Z_COUNT zombies. Reaping failed!"
        exit 1
    fi

    exit 0
EOF
