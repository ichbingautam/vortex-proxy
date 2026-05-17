#!/usr/bin/env bash
# Vortex Proxy Load Testing Script (via Vegeta)
# Usage: ./perf-test.sh [duration] [rate] [target_url]
# Example: ./perf-test.sh 30s 10000 https://localhost:8443

set -e

DURATION=${1:-"30s"}
RATE=${2:-"5000"}
TARGET=${3:-"http://127.0.0.1:8443"}

if ! command -v vegeta &> /dev/null; then
    echo "Error: vegeta is not installed."
    echo "Install it via: brew install vegeta (macOS) or go install github.com/tsenart/vegeta@latest"
    exit 1
fi

echo "=========================================================="
echo "🚀 Initiating Vortex Proxy Performance Test"
echo "Target:   $TARGET"
echo "Rate:     $RATE requests per second"
echo "Duration: $DURATION"
echo "=========================================================="

# Create an output directory for the results
mkdir -p results

REPORT_BIN="results/load-test.bin"
REPORT_TXT="results/load-test-report.txt"

# Run Vegeta attack
echo "GET $TARGET" | vegeta attack -duration="$DURATION" -rate="$RATE" -insecure > "$REPORT_BIN"

echo ""
echo "📊 Test Complete. Generating Report..."
echo "=========================================================="

# Generate and display the text report
vegeta report -type=text "$REPORT_BIN" > "$REPORT_TXT"
cat "$REPORT_TXT"

echo "=========================================================="
echo "A detailed report has been saved to: $REPORT_TXT"
