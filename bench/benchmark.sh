#!/usr/bin/env bash
# Benchmark script for rmbrr vs common Unix deletion methods
# Usage: ./benchmark.sh

set -e

# Configuration
BENCH_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEST_ROOT="${TEST_ROOT:-/tmp/rmbrr_bench_test}"
RESULTS_FILE="${BENCH_DIR}/benchmark_results_linux.txt"
RMBRR="${BENCH_DIR}/../target/release/rmbrr"
SOURCE_DIR="/tmp/rmbrr_bench_node_modules"

# Colors
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
GRAY='\033[0;90m'
NC='\033[0m' # No Color

# Ensure the release binary exists
if [ ! -f "$RMBRR" ]; then
    echo -e "${YELLOW}Building rmbrr in release mode...${NC}"
    cd "${BENCH_DIR}/.."
    cargo build --release
    cd "${BENCH_DIR}"
fi

# Create test root directory
if [ -d "$TEST_ROOT" ]; then
    echo -e "${YELLOW}Cleaning up old test directories...${NC}"
    rm -rf "$TEST_ROOT"
fi
mkdir -p "$TEST_ROOT"

echo -e "${CYAN}=== rmbrr Benchmark Suite (Linux) ===${NC}"
echo ""

# Step 1: Install the nightmare node_modules directly on Linux fs
echo -e "${GREEN}[1/2] Creating nightmare node_modules...${NC}"

if [ ! -d "$SOURCE_DIR" ]; then
    echo -e "${YELLOW}  Installing dependencies directly to /tmp (this will take a while)...${NC}"

    # Create temp directory and copy package.json there
    mkdir -p "$SOURCE_DIR"
    cp "${BENCH_DIR}/package.json" "$SOURCE_DIR/"

    # Install in /tmp
    cd "$SOURCE_DIR"
    npm install --loglevel=error

    # Move node_modules up one level and remove temp package files
    if [ -d "node_modules" ]; then
        mv node_modules node_modules_temp
        mv node_modules_temp/* .
        rm -rf node_modules_temp
    fi
    rm -f package.json package-lock.json

    cd - > /dev/null

    echo -e "${GREEN}  Installed to ${SOURCE_DIR}${NC}"
fi

# Count files and directories
echo -e "${YELLOW}  Analyzing node_modules structure...${NC}"
FILE_COUNT=$(find "$SOURCE_DIR" -type f 2>/dev/null | wc -l)
DIR_COUNT=$(find "$SOURCE_DIR" -type d 2>/dev/null | wc -l)
TOTAL_SIZE=$(du -sm "$SOURCE_DIR" 2>/dev/null | cut -f1)

echo ""
echo -e "${CYAN}Test dataset:${NC}"
echo -e "  Files:       ${FILE_COUNT}"
echo -e "  Directories: ${DIR_COUNT}"
echo -e "  Total size:  ${TOTAL_SIZE} MB"
echo ""

# Initialize results
RESULTS=()
RESULTS+=("=== rmbrr Benchmark Results (Linux) ===")
RESULTS+=("")
RESULTS+=("Test dataset:")
RESULTS+=("  Files:       ${FILE_COUNT}")
RESULTS+=("  Directories: ${DIR_COUNT}")
RESULTS+=("  Total size:  ${TOTAL_SIZE} MB")
RESULTS+=("")
RESULTS+=("Results:")
RESULTS+=("")

# Thread counts to test
THREAD_COUNTS=(1 2 4 8 16 32)

# Test rmbrr with different thread counts
for THREADS in "${THREAD_COUNTS[@]}"; do
    TEST_DIR="${TEST_ROOT}/test_rmbrr_${THREADS}"

    echo -e "${YELLOW}Testing: rmbrr (${THREADS} threads)${NC}"
    echo -e "${GRAY}  Copying test data...${NC}"

    # Copy node_modules for this test
    cp -r "$SOURCE_DIR" "$TEST_DIR"

    # Verify copy
    if [ ! -d "$TEST_DIR" ]; then
        echo -e "${RED}  ERROR: Failed to copy test directory${NC}"
        continue
    fi

    echo -e "${GRAY}  Running deletion...${NC}"

    # Measure deletion time
    START=$(date +%s%3N)
    "$RMBRR" "$TEST_DIR" --silent --threads "$THREADS" 2>/dev/null || true
    END=$(date +%s%3N)

    # Verify deletion
    if [ ! -d "$TEST_DIR" ]; then
        TIME_MS=$((END - START))
        TIME_SEC=$(echo "scale=3; $TIME_MS / 1000" | bc)
        echo -e "${GREEN}  ✓ Completed in ${TIME_SEC}s (${TIME_MS}ms)${NC}"
        RESULTS+=("rmbrr (${THREADS} threads): ${TIME_MS}ms (${TIME_SEC}s)")
    else
        echo -e "${RED}  ✗ Failed (directory still exists)${NC}"
        RESULTS+=("rmbrr (${THREADS} threads): FAILED (incomplete deletion)")
        rm -rf "$TEST_DIR"
    fi

    echo ""
    sleep 0.5
done

# Test other methods
declare -A METHODS=(
    ["rm -rf"]="rm -rf"
)

# Check if rimraf is available
if command -v npx &> /dev/null && npx rimraf --help &> /dev/null; then
    METHODS["rimraf (Node.js)"]="npx rimraf"
fi

for METHOD_NAME in "${!METHODS[@]}"; do
    TEST_DIR="${TEST_ROOT}/test_$(echo "$METHOD_NAME" | tr ' ' '_' | tr -cd '[:alnum:]_')"

    echo -e "${YELLOW}Testing: ${METHOD_NAME}${NC}"
    echo -e "${GRAY}  Copying test data...${NC}"

    # Copy node_modules for this test
    cp -r "$SOURCE_DIR" "$TEST_DIR"

    # Verify copy
    if [ ! -d "$TEST_DIR" ]; then
        echo -e "${RED}  ERROR: Failed to copy test directory${NC}"
        continue
    fi

    echo -e "${GRAY}  Running deletion...${NC}"

    # Measure deletion time
    START=$(date +%s%3N)
    eval "${METHODS[$METHOD_NAME]} \"$TEST_DIR\"" 2>/dev/null || true
    END=$(date +%s%3N)

    # Verify deletion
    if [ ! -d "$TEST_DIR" ]; then
        TIME_MS=$((END - START))
        TIME_SEC=$(echo "scale=3; $TIME_MS / 1000" | bc)
        echo -e "${GREEN}  ✓ Completed in ${TIME_SEC}s (${TIME_MS}ms)${NC}"
        RESULTS+=("${METHOD_NAME}: ${TIME_MS}ms (${TIME_SEC}s)")
    else
        echo -e "${RED}  ✗ Failed (directory still exists)${NC}"
        RESULTS+=("${METHOD_NAME}: FAILED (incomplete deletion)")
        rm -rf "$TEST_DIR"
    fi

    echo ""
    sleep 0.5
done

# Calculate relative performance
echo -e "${CYAN}=== Results Summary ===${NC}"
echo ""

# Find best rmbrr time
RMBRR_TIME=""
for LINE in "${RESULTS[@]}"; do
    if [[ "$LINE" =~ ^rmbrr\ \(.+\ threads\):\ ([0-9]+)ms ]]; then
        CURRENT_TIME="${BASH_REMATCH[1]}"
        if [ -z "$RMBRR_TIME" ] || [ "$CURRENT_TIME" -lt "$RMBRR_TIME" ]; then
            RMBRR_TIME="$CURRENT_TIME"
        fi
    fi
done

# Display results with relative performance
for LINE in "${RESULTS[@]}"; do
    if [[ "$LINE" =~ ^(.+):\ ([0-9]+)ms ]]; then
        NAME="${BASH_REMATCH[1]}"
        TIME_MS="${BASH_REMATCH[2]}"
        TIME_SEC=$(echo "scale=3; $TIME_MS / 1000" | bc)

        # Compare to best rmbrr time if it's not a rmbrr variant
        if [ -n "$RMBRR_TIME" ] && [[ ! "$NAME" =~ ^rmbrr\ \( ]]; then
            RATIO=$(echo "scale=2; $TIME_MS / $RMBRR_TIME" | bc)
            echo "${NAME}: ${TIME_SEC}s (${RATIO}x slower)"
            RESULTS+=("${NAME}: ${TIME_SEC}s (${RATIO}x slower than best rmbrr)")
        else
            echo "${NAME}: ${TIME_SEC}s"
        fi
    elif [[ "$LINE" =~ FAILED ]]; then
        echo -e "${RED}${LINE}${NC}"
    elif [ -n "$LINE" ]; then
        echo -e "${GRAY}${LINE}${NC}"
    fi
done

echo ""

# Save results to file
printf "%s\n" "${RESULTS[@]}" > "$RESULTS_FILE"
echo -e "${GREEN}Results saved to: ${RESULTS_FILE}${NC}"

# Cleanup
echo ""
echo -e "${YELLOW}Cleaning up test directories...${NC}"
rm -rf "$TEST_ROOT"

echo -e "${GREEN}Benchmark complete!${NC}"
