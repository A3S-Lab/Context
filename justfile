# A3S Context - Justfile

default:
    @just --list

# AI-powered commit message
cz:
    @bash .scripts/generate-commit-message.sh

# ============================================================================
# Build
# ============================================================================

# Build in release mode
build:
    cargo build --release

# Build in debug mode
build-debug:
    cargo build

# ============================================================================
# Test (unified command with progress display)
# ============================================================================

# Run all tests with progress display and module breakdown
test:
    #!/usr/bin/env bash
    set -e

    # Colors
    BOLD='\033[1m'
    GREEN='\033[0;32m'
    BLUE='\033[0;34m'
    CYAN='\033[0;36m'
    YELLOW='\033[0;33m'
    RED='\033[0;31m'
    DIM='\033[2m'
    RESET='\033[0m'

    # Counters
    TOTAL_PASSED=0
    TOTAL_FAILED=0
    TOTAL_IGNORED=0

    print_header() {
        echo ""
        echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
        echo -e "${BOLD}  $1${RESET}"
        echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    }

    # Extract module test counts from cargo test output
    extract_module_counts() {
        local output="$1"
        echo "$output" | grep -E "^test .+::.+ \.\.\. ok$" | \
            sed 's/^test \([^:]*\)::.*/\1/' | \
            sort | uniq -c | sort -rn | \
            while read count module; do
                printf "      ${DIM}%-20s %3d tests${RESET}\n" "$module" "$count"
            done
    }

    print_header "🧪 A3S Context Test Suite"
    echo ""
    echo -ne "${CYAN}▶${RESET} ${BOLD}a3s-context${RESET} "

    # Run tests and capture output
    if OUTPUT=$(cargo test --all-features 2>&1); then
        TEST_EXIT=0
    else
        TEST_EXIT=1
    fi

    # Extract test results
    RESULT_LINE=$(echo "$OUTPUT" | grep -E "^test result:" | tail -1)
    if [ -n "$RESULT_LINE" ]; then
        PASSED=$(echo "$RESULT_LINE" | grep -oE '[0-9]+ passed' | grep -oE '[0-9]+' || echo "0")
        FAILED=$(echo "$RESULT_LINE" | grep -oE '[0-9]+ failed' | grep -oE '[0-9]+' || echo "0")
        IGNORED=$(echo "$RESULT_LINE" | grep -oE '[0-9]+ ignored' | grep -oE '[0-9]+' || echo "0")

        TOTAL_PASSED=$((TOTAL_PASSED + PASSED))
        TOTAL_FAILED=$((TOTAL_FAILED + FAILED))
        TOTAL_IGNORED=$((TOTAL_IGNORED + IGNORED))

        if [ "$FAILED" -gt 0 ]; then
            echo -e "${RED}✗${RESET} ${DIM}$PASSED passed, $FAILED failed${RESET}"
            echo "$OUTPUT" | grep -E "^test .* FAILED$" | sed 's/^/    /'
        else
            echo -e "${GREEN}✓${RESET} ${DIM}$PASSED passed${RESET}"
            # Show module breakdown for crates with many tests
            if [ "$PASSED" -gt 10 ]; then
                extract_module_counts "$OUTPUT"
            fi
        fi
    else
        # No tests found or compilation error
        if echo "$OUTPUT" | grep -q "error\[E"; then
            echo -e "${RED}✗${RESET} ${DIM}compile error${RESET}"
            echo "$OUTPUT" | grep -E "^error" | head -3 | sed 's/^/    /'
        elif [ "$TEST_EXIT" -ne 0 ]; then
            echo -e "${RED}✗${RESET} ${DIM}failed${RESET}"
        else
            echo -e "${YELLOW}○${RESET} ${DIM}no tests${RESET}"
        fi
    fi

    # Summary
    echo ""
    echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"

    if [ "$TOTAL_FAILED" -gt 0 ]; then
        echo -e "  ${RED}${BOLD}✗ FAILED${RESET}  ${GREEN}$TOTAL_PASSED passed${RESET}  ${RED}$TOTAL_FAILED failed${RESET}  ${YELLOW}$TOTAL_IGNORED ignored${RESET}"
        echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
        exit 1
    else
        echo -e "  ${GREEN}${BOLD}✓ PASSED${RESET}  ${GREEN}$TOTAL_PASSED passed${RESET}  ${YELLOW}$TOTAL_IGNORED ignored${RESET}"
        echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    fi
    echo ""

# Run tests without progress (raw cargo output)
test-raw:
    cargo test --all-features

# Run tests with verbose output
test-v:
    cargo test --all-features -- --nocapture

# Run specific test
test-one TEST:
    cargo test {{TEST}} -- --nocapture

# ============================================================================
# Test Subsets
# ============================================================================

# Test pathway module
test-pathway:
    cargo test --all-features -- pathway::tests

# Test storage module
test-storage:
    cargo test --all-features -- storage::tests

# Test retrieval module
test-retrieval:
    cargo test --all-features -- retrieval::tests

# Test session module
test-session:
    cargo test --all-features -- session::tests

# Test config module
test-config:
    cargo test --all-features -- config::tests

# Run integration tests
test-integration:
    cargo test --all-features --test integration_test

# ============================================================================
# Coverage (requires: cargo install cargo-llvm-cov, brew install lcov)
# ============================================================================

# Test with coverage - shows real-time test progress + module coverage
test-cov:
    #!/usr/bin/env bash
    set -e

    # Colors
    BOLD='\033[1m'
    GREEN='\033[0;32m'
    BLUE='\033[0;34m'
    CYAN='\033[0;36m'
    YELLOW='\033[0;33m'
    RED='\033[0;31m'
    DIM='\033[2m'
    RESET='\033[0m'

    # Clear line and move cursor
    CLEAR_LINE='\033[2K'

    print_header() {
        echo ""
        echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
        echo -e "${BOLD}  $1${RESET}"
        echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    }

    print_header "🧪 A3S Context Test Suite with Coverage"
    echo ""
    echo -e "${CYAN}▶${RESET} ${BOLD}a3s-context${RESET}"
    echo ""

    # Temp files for tracking
    tmp_dir="/tmp/test_cov_context_$$"
    mkdir -p "$tmp_dir"
    touch "$tmp_dir/module_counts"

    # Clean previous coverage data
    cargo llvm-cov clean --workspace 2>/dev/null || true

    # Run tests with coverage
    {
        cargo llvm-cov --all-features --workspace 2>&1
    } | {
        total_passed=0
        total_failed=0

        while IFS= read -r line; do
            # Check if it's a test result line
            if [[ "$line" =~ ^test\ ([a-z_]+)::.*\.\.\.\ (ok|FAILED)$ ]]; then
                module="${BASH_REMATCH[1]}"
                result="${BASH_REMATCH[2]}"

                if [ "$result" = "ok" ]; then
                    total_passed=$((total_passed + 1))
                    count=$(grep "^${module} " "$tmp_dir/module_counts" 2>/dev/null | awk '{print $2}' || echo "0")
                    count=$((count + 1))
                    grep -v "^${module} " "$tmp_dir/module_counts" > "$tmp_dir/module_counts.tmp" 2>/dev/null || true
                    echo "$module $count" >> "$tmp_dir/module_counts.tmp"
                    mv "$tmp_dir/module_counts.tmp" "$tmp_dir/module_counts"
                else
                    total_failed=$((total_failed + 1))
                fi

                echo -ne "\r${CLEAR_LINE}      ${DIM}Running:${RESET} ${module}::... ${GREEN}${total_passed}${RESET} passed"
                [ "$total_failed" -gt 0 ] && echo -ne " ${RED}${total_failed}${RESET} failed"

            elif [[ "$line" =~ ^[[:space:]]*Compiling ]]; then
                echo -ne "\r${CLEAR_LINE}      ${DIM}Compiling...${RESET}"
            elif [[ "$line" =~ ^[[:space:]]*Running ]]; then
                echo -ne "\r${CLEAR_LINE}      ${DIM}Running tests...${RESET}"
            elif [[ "$line" =~ ^[a-z_]+.*\.rs[[:space:]] ]]; then
                echo "$line" >> "$tmp_dir/coverage_lines"
            elif [[ "$line" =~ ^TOTAL ]]; then
                echo "$line" >> "$tmp_dir/total_line"
            fi
        done

        echo "$total_passed" > "$tmp_dir/total_passed"
        echo "$total_failed" > "$tmp_dir/total_failed"
    }

    # Clear progress line
    echo -ne "\r${CLEAR_LINE}"

    # Read results
    total_passed=$(cat "$tmp_dir/total_passed" 2>/dev/null || echo "0")
    total_failed=$(cat "$tmp_dir/total_failed" 2>/dev/null || echo "0")

    # Show final test result
    if [ "$total_failed" -gt 0 ]; then
        echo -e "      ${RED}✗${RESET} ${total_passed} passed, ${RED}${total_failed} failed${RESET}"
    else
        echo -e "      ${GREEN}✓${RESET} ${total_passed} tests passed"
    fi
    echo ""

    # Parse coverage data and aggregate by module
    if [ -f "$tmp_dir/coverage_lines" ]; then
        awk '
        {
            file=$1; lines=$8; missed=$9
            n = split(file, parts, "/")
            if (n > 1) {
                module = parts[1]
            } else {
                gsub(/\.rs$/, "", file)
                module = file
            }
            total_lines[module] += lines
            total_missed[module] += missed
        }
        END {
            for (m in total_lines) {
                if (total_lines[m] > 0) {
                    covered = total_lines[m] - total_missed[m]
                    pct = (covered / total_lines[m]) * 100
                    printf "%s %.1f %d\n", m, pct, total_lines[m]
                }
            }
        }' "$tmp_dir/coverage_lines" | sort -t' ' -k2 -rn > "$tmp_dir/cov_agg"

        # Display coverage results with test counts
        echo -e "      ${BOLD}Module               Tests   Coverage${RESET}"
        echo -e "      ${DIM}──────────────────────────────────────────────${RESET}"

        while read module pct lines; do
            tests=$(grep "^${module} " "$tmp_dir/module_counts" 2>/dev/null | awk '{print $2}' || echo "0")
            [ -z "$tests" ] && tests=0

            num=${pct%.*}
            if [ "$num" -ge 90 ]; then
                cov_color="${GREEN}${pct}%${RESET}"
            elif [ "$num" -ge 70 ]; then
                cov_color="${YELLOW}${pct}%${RESET}"
            else
                cov_color="${RED}${pct}%${RESET}"
            fi
            echo -e "      $(printf '%-18s' "$module") $(printf '%4d' "$tests")   ${cov_color} ${DIM}($lines lines)${RESET}"
        done < "$tmp_dir/cov_agg"

        # Print total
        if [ -f "$tmp_dir/total_line" ]; then
            total_cov=$(cat "$tmp_dir/total_line" | awk '{print $4}' | tr -d '%')
            total_lines=$(cat "$tmp_dir/total_line" | awk '{print $8}')
            echo -e "      ${DIM}──────────────────────────────────────────────${RESET}"

            num=${total_cov%.*}
            if [ "$num" -ge 90 ]; then
                cov_color="${GREEN}${BOLD}${total_cov}%${RESET}"
            elif [ "$num" -ge 70 ]; then
                cov_color="${YELLOW}${BOLD}${total_cov}%${RESET}"
            else
                cov_color="${RED}${BOLD}${total_cov}%${RESET}"
            fi
            echo -e "      ${BOLD}$(printf '%-18s' "TOTAL") $(printf '%4d' "$total_passed")${RESET}   ${cov_color} ${DIM}($total_lines lines)${RESET}"
        fi
    fi

    # Cleanup
    rm -rf "$tmp_dir"
    echo ""
    echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    echo ""

# Coverage with pretty terminal output
cov:
    #!/usr/bin/env bash
    set -e
    COV_FILE="/tmp/a3s-context-coverage.lcov"
    echo "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓"
    echo "┃                    🧪 Running Tests with Coverage                     ┃"
    echo "┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛"
    cargo llvm-cov clean --workspace
    cargo llvm-cov --all-features --workspace --lcov --output-path "$COV_FILE" 2>&1 | grep -E "^test result"
    echo ""
    echo "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓"
    echo "┃                         📊 Coverage Report                            ┃"
    echo "┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛"
    lcov --summary "$COV_FILE" 2>&1
    rm -f "$COV_FILE"

# Coverage for specific module
cov-module MOD:
    cargo llvm-cov --all-features -- {{MOD}}::

# Coverage with HTML report (opens in browser)
cov-html:
    #!/usr/bin/env bash
    set -e
    cargo llvm-cov clean --workspace
    cargo llvm-cov --all-features --workspace --html
    echo ""
    echo "HTML report generated at: target/llvm-cov/html/index.html"
    if command -v open &> /dev/null; then
        open target/llvm-cov/html/index.html
    elif command -v xdg-open &> /dev/null; then
        xdg-open target/llvm-cov/html/index.html
    else
        echo "Open target/llvm-cov/html/index.html in your browser"
    fi

# Coverage with detailed file-by-file table
cov-table:
    cargo llvm-cov clean --workspace
    cargo llvm-cov --all-features --workspace

# Coverage for CI (generates lcov.info)
cov-ci:
    cargo llvm-cov clean --workspace
    cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info

# Clean coverage data
cov-clean:
    cargo llvm-cov clean --workspace
    rm -f lcov.info

# ============================================================================
# Code Quality
# ============================================================================

# Run clippy lints
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Format code
fmt:
    cargo fmt --all

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# CI checks (fmt + lint + test)
ci:
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test --all-features

# ============================================================================
# CLI
# ============================================================================

# Run the CLI tool
run *ARGS:
    cargo run --bin a3s-ctx -- {{ARGS}}

# ============================================================================
# Utilities
# ============================================================================

# Clean build artifacts
clean:
    cargo clean
    rm -f lcov.info
    rm -rf target/llvm-cov

# Check project (fast compile check)
check:
    cargo check --all-features

# Run benchmarks
bench:
    cargo bench

# Watch for changes and run tests
watch:
    cargo watch -x test

# Generate documentation
doc:
    cargo doc --no-deps --open

# Update dependencies
update:
    cargo update

# Install the binary
install:
    cargo install --path .

# Show project statistics
stats:
    #!/usr/bin/env bash
    echo ""
    echo "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓"
    echo "┃                       📊 Project Statistics                           ┃"
    echo "┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛"
    echo ""
    echo "Lines of code:"
    find src -name "*.rs" | xargs wc -l | tail -1
    echo ""
    echo "Test files:"
    find tests -name "*.rs" 2>/dev/null | wc -l || echo "0"
    echo ""
    echo "Dependencies:"
    cargo tree --depth 1 | wc -l
    echo ""

# ============================================================================
# Publish
# ============================================================================

# Publish to crates.io (with all checks)
publish:
    #!/usr/bin/env bash
    set -e

    # Colors
    BOLD='\033[1m'
    GREEN='\033[0;32m'
    BLUE='\033[0;34m'
    YELLOW='\033[0;33m'
    RED='\033[0;31m'
    DIM='\033[2m'
    RESET='\033[0m'

    print_step() {
        echo -e "${BLUE}▶${RESET} ${BOLD}$1${RESET}"
    }

    print_success() {
        echo -e "${GREEN}✓${RESET} $1"
    }

    print_error() {
        echo -e "${RED}✗${RESET} $1"
        exit 1
    }

    echo ""
    echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    echo -e "${BOLD}  📦 Publishing a3s_context to crates.io${RESET}"
    echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    echo ""

    # Show current version
    VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
    echo -e "  ${DIM}Version:${RESET} ${BOLD}${VERSION}${RESET}"
    echo ""

    # Step 1: Format check
    print_step "Checking formatting..."
    if cargo fmt --all -- --check; then
        print_success "Formatting OK"
    else
        print_error "Formatting check failed. Run 'just fmt' first."
    fi

    # Step 2: Lint
    print_step "Running clippy..."
    if cargo clippy --all-targets --all-features -- -D warnings; then
        print_success "Clippy OK"
    else
        print_error "Clippy check failed. Fix warnings first."
    fi

    # Step 3: Test
    print_step "Running tests..."
    if cargo test --all-features; then
        print_success "Tests OK"
    else
        print_error "Tests failed."
    fi

    # Step 4: Dry run
    print_step "Verifying package..."
    if cargo publish --dry-run; then
        print_success "Package verification OK"
    else
        print_error "Package verification failed."
    fi

    # Step 5: Publish
    print_step "Publishing to crates.io..."
    if cargo publish; then
        echo ""
        echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
        echo -e "  ${GREEN}${BOLD}✓ Successfully published a3s_context v${VERSION}${RESET}"
        echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    else
        print_error "Publish failed."
    fi
    echo ""

# Publish dry-run (verify without publishing)
publish-dry:
    #!/usr/bin/env bash
    set -e
    echo ""
    echo "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓"
    echo "┃                  📦 Publish Dry Run (a3s_context)                      ┃"
    echo "┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛"
    echo ""
    VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
    echo "Version: ${VERSION}"
    echo ""
    cargo publish --dry-run
    echo ""
    echo "✓ Dry run successful. Ready to publish with 'just publish'"
    echo ""

# Show current version
version:
    @grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'
