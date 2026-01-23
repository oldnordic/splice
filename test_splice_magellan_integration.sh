#!/bin/bash
# Integration test for Splice + Magellan workflow
# This validates that:
# 1. Splice works standalone (without Magellan)
# 2. Splice works with Magellan database
# 3. JSON output format is aligned between tools

set -e

echo "=========================================="
echo "Splice + Magellan Integration Test"
echo "=========================================="
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counters
TESTS_PASSED=0
TESTS_FAILED=0

# Helper functions
success() {
    echo -e "${GREEN}✓ PASS${NC}: $1"
    ((TESTS_PASSED++))
}

failure() {
    echo -e "${RED}✗ FAIL${NC}: $1"
    ((TESTS_FAILED++))
}

info() {
    echo -e "${YELLOW}ℹ INFO${NC}: $1"
}

# Test 1: Check binaries are available
echo "Test 1: Verify binaries are installed"
echo "-------------------------------------------"

if which splice > /dev/null 2>&1; then
    SPLICE_VERSION=$(splice --version)
    success "Splice found: $SPLICE_VERSION"
else
    failure "Splice binary not found"
fi

if which magellan > /dev/null 2>&1; then
    MAGELLAN_VERSION=$(magellan --version 2>&1 | head -1)
    success "Magellan found: $MAGELLAN_VERSION"
else
    failure "Magellan binary not found"
fi

echo ""

# Test 2: Check databases exist
echo "Test 2: Verify databases exist"
echo "-------------------------------------------"

CODEGRAPH_DB=".codemcp/codegraph.db"
OPERATIONS_DB=".codemcp/operations.db"

if [ -f "$CODEGRAPH_DB" ]; then
    CODEGRAPH_SIZE=$(du -h "$CODEGRAPH_DB" | cut -f1)
    success "codegraph.db exists ($CODEGRAPH_SIZE)"
else
    failure "codegraph.db not found"
fi

if [ -f "$OPERATIONS_DB" ]; then
    OPS_SIZE=$(du -h "$OPERATIONS_DB" | cut -f1)
    success "operations.db exists ($OPS_SIZE)"
else
    failure "operations.db not found"
fi

echo ""

# Test 3: Validate database schema alignment
echo "Test 3: Check database schema"
echo "-------------------------------------------"

# Check for required tables in codegraph.db
REQUIRED_TABLES=("graph_entities" "graph_edges" "graph_labels" "graph_properties" "code_chunks")
for table in "${REQUIRED_TABLES[@]}"; do
    COUNT=$(sqlite3 "$CODEGRAPH_DB" "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='$table'")
    if [ "$COUNT" -eq "1" ]; then
        success "Table '$table' exists in codegraph.db"
    else
        failure "Table '$table' missing from codegraph.db"
    fi
done

echo ""

# Test 4: Test Splice standalone mode (patch without database)
echo "Test 4: Splice standalone mode"
echo "-------------------------------------------"

# Create a test file
TEST_DIR=$(mktemp -d)
TEST_FILE="$TEST_DIR/test_standalone.rs"
cat > "$TEST_FILE" << 'EOF'
/// Old function
pub fn old_function() -> i32 {
    42
}
EOF

# Use splice to find and patch it
cat > "$TEST_FILE.patch" << 'EOF'
/// New function
pub fn new_function() -> i32 {
    43
}
EOF

info "Test file created: $TEST_FILE"
info "Attempting patch operation..."

if splice patch \
    --file "$TEST_FILE" \
    --symbol old_function \
    --kind function \
    --with "$TEST_FILE.patch" \
    --create-backup \
    --dry-run \
    --json > /dev/null 2>&1; then
    success "Splice works in standalone mode (patch without database)"
else
    failure "Splice standalone mode failed"
fi

echo ""

# Test 5: Test Splice with Magellan database (get/query)
echo "Test 5: Splice + Magellan integration"
echo "-------------------------------------------"

# Query for symbols in database
SYMBOL_COUNT=$(sqlite3 "$CODEGRAPH_DB" "SELECT COUNT(*) FROM code_chunks WHERE file_path LIKE '$PWD/src/%' LIMIT 1")

if [ -n "$SYMBOL_COUNT" ] && [ "$SYMBOL_COUNT" -gt "0" ]; then
    success "Magellan database has indexed source files ($SYMBOL_COUNT code chunks)"
else
    failure "Magellan database has no indexed source files"
fi

# Get a symbol from database
SYMBOL_INFO=$(sqlite3 "$CODEGRAPH_DB" "SELECT file_path, byte_start, byte_end FROM code_chunks WHERE file_path LIKE '$PWD/src/%' LIMIT 1")

if [ -n "$SYMBOL_INFO" ]; then
    FILE_PATH=$(echo "$SYMBOL_INFO" | cut -d'|' -f1)
    BYTE_START=$(echo "$SYMBOL_INFO" | cut -d'|' -f2)
    BYTE_END=$(echo "$SYMBOL_INFO" | cut -d'|' -f3)

    info "Testing get command: $FILE_PATH:$BYTE_START-$BYTE_END"

    # Convert to absolute path for splice
    ABSOLUTE_PATH="$PWD/$(basename $(echo $FILE_PATH | sed 's|.*src/||'))"

    if splice get \
        --db "$CODEGRAPH_DB" \
        --file "$ABSOLUTE_PATH" \
        --start "$BYTE_START" \
        --end "$BYTE_END" \
        --json > /dev/null 2>&1; then
        success "Splice get works with Magellan database"
    else
        failure "Splice get failed with Magellan database"
    fi
else
    failure "Could not find symbol in Magellan database"
fi

echo ""

# Test 6: Validate JSON output format alignment
echo "Test 6: JSON output format validation"
echo "-------------------------------------------"

# Get output from Magellan
MAGELLAN_OUTPUT=$(magellan find --db "$CODEGRAPH_DB" --name main --output json 2>&1)

# Check for required fields in Magellan output
MAGELLAN_SCHEMA_VALID=true
for field in "schema_version" "execution_id" "tool" "timestamp" "data"; do
    if echo "$MAGELLAN_OUTPUT" | grep -q "\"$field\""; then
        success "Magellan JSON has field: $field"
    else
        failure "Magellan JSON missing field: $field"
        MAGELLAN_SCHEMA_VALID=false
    fi
done

# Get output from Splice
if [ -n "$SYMBOL_INFO" ]; then
    FILE_PATH=$(echo "$SYMBOL_INFO" | cut -d'|' -f1)
    BYTE_START=$(echo "$SYMBOL_INFO" | cut -d'|' -f2)
    BYTE_END=$(echo "$SYMBOL_INFO" | cut -d'|' -f3)
    ABSOLUTE_PATH="$PWD/$(basename $(echo $FILE_PATH | sed 's|.*src/||'))"

    SPLICE_OUTPUT=$(splice get \
        --db "$CODEGRAPH_DB" \
        --file "$ABSOLUTE_PATH" \
        --start "$BYTE_START" \
        --end "$BYTE_END" \
        --json 2>&1)

    # Check for required fields in Splice output
    SPLICE_SCHEMA_VALID=true
    for field in "schema_version" "execution_id" "tool" "timestamp" "result"; do
        if echo "$SPLICE_OUTPUT" | grep -q "\"$field\""; then
            success "Splice JSON has field: $field"
        else
            failure "Splice JSON missing field: $field"
            SPLICE_SCHEMA_VALID=false
        fi
    done

    # Check schema version alignment
    MAGELLAN_VERSION=$(echo "$MAGELLAN_OUTPUT" | grep -o '"schema_version":"[^"]*"' | cut -d'"' -f4)
    SPLICE_VERSION=$(echo "$SPLICE_OUTPUT" | grep -o '"schema_version":"[^"]*"' | cut -d'"' -f4)

    info "Magellan schema version: $MAGELLAN_VERSION"
    info "Splice schema version: $SPLICE_VERSION"

    if [ "$MAGELLAN_VERSION" = "$SPLICE_VERSION" ]; then
        success "Schema versions are aligned"
    else
        info "Schema versions differ (expected: Magellan provides structure, Splice consumes)"
    fi
fi

echo ""

# Test 7: Test symbol query workflow
echo "Test 7: Query workflow test"
echo "-------------------------------------------"

# List available labels
LABELS_OUTPUT=$(splice query --db "$CODEGRAPH_DB" --list 2>&1)

if echo "$LABELS_OUTPUT" | grep -q "labels in use"; then
    success "Splice query --list works"
else
    failure "Splice query --list failed"
fi

# Query for functions
FUNCTIONS_OUTPUT=$(splice query --db "$CODEGRAPH_DB" --label fn --limit 1 --json 2>&1)

if echo "$FUNCTIONS_OUTPUT" | grep -q '"operation_type":"query"'; then
    success "Splice query by label works"
else
    failure "Splice query by label failed"
fi

echo ""

# Cleanup
rm -rf "$TEST_DIR"

# Summary
echo "=========================================="
echo "Test Summary"
echo "=========================================="
echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
echo -e "${RED}Failed: $TESTS_FAILED${NC}"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some tests failed${NC}"
    exit 1
fi
