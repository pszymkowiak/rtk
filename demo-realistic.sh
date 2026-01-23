#!/bin/bash
# Realistic human typing simulator for asciinema demo
# All 17 rtk commands demonstrated

# ANSI colors
RESET="\033[0m"
BOLD="\033[1m"
GREEN="\033[32m"
BLUE="\033[34m"
GRAY="\033[90m"
YELLOW="\033[33m"

# Typing simulation with human-like timing
human_type() {
    local text="$1"
    local i=0
    local len=${#text}
    local error_chance=6

    while [ $i -lt $len ]; do
        char="${text:$i:1}"

        # Random error injection (~6% chance)
        if [ $((RANDOM % 100)) -lt $error_chance ] && [ "$char" != " " ] && [ "$char" != "-" ]; then
            wrong_chars="qwertyuiopasdfghjklzxcvbnm"
            wrong_char="${wrong_chars:$((RANDOM % 26)):1}"
            printf "%s" "$wrong_char"
            sleep $(echo "scale=3; (100 + $RANDOM % 300) / 1000" | bc)
            printf "\b \b"
            sleep 0.05
        fi

        printf "%s" "$char"

        case "$char" in
            " ")
                sleep $(echo "scale=3; (80 + $RANDOM % 120) / 1000" | bc)
                ;;
            "." | "," | "-" | "/" | "\"")
                sleep $(echo "scale=3; (100 + $RANDOM % 150) / 1000" | bc)
                ;;
            *)
                case "$char" in
                    [etaoins])
                        sleep $(echo "scale=3; (40 + $RANDOM % 60) / 1000" | bc)
                        ;;
                    [rhldcu])
                        sleep $(echo "scale=3; (50 + $RANDOM % 70) / 1000" | bc)
                        ;;
                    *)
                        sleep $(echo "scale=3; (60 + $RANDOM % 90) / 1000" | bc)
                        ;;
                esac
                ;;
        esac
        i=$((i + 1))
    done
}

type_cmd() {
    printf "${GREEN}\$${RESET} "
    human_type "$1"
    echo ""
    sleep 0.3
}

think() {
    sleep $(echo "scale=2; 0.5 + $RANDOM % 10 / 10" | bc)
}

section() {
    echo ""
    printf "${BOLD}${BLUE}━━━ $1 ━━━${RESET}\n"
    sleep 0.6
}

# Start
clear
echo -e "${BOLD}${YELLOW}rtk${RESET} ${BOLD}- Rust Token Killer${RESET}"
echo -e "${GRAY}All 21 commands demo | Saves 60-90% tokens${RESET}"
echo ""
sleep 2

#######################################
# FILES COMMANDS
#######################################

section "1. rtk ls - Directory listing"
think
type_cmd "rtk ls . -d 2"
rtk ls . -d 2
sleep 2

section "2. rtk read - Smart file reading"
think
type_cmd "rtk read src/main.rs -l aggressive -m 12"
rtk read src/main.rs -l aggressive --max-lines 12
sleep 2

section "3. rtk smart - Heuristic summary"
think
type_cmd "rtk smart src/git.rs"
rtk smart src/git.rs
sleep 2

section "4. rtk find - Find files"
think
type_cmd "rtk find \"*.rs\" src/"
rtk find "*.rs" src/ --max 10
sleep 2

section "5. rtk diff - Ultra-condensed diff"
think
type_cmd "rtk diff src/main.rs src/git.rs"
rtk diff src/main.rs src/git.rs 2>&1 | head -15
sleep 2

section "6. rtk grep - Compact search"
think
type_cmd "rtk grep \"Result\" src/ -m 8"
rtk grep "Result" src/ --max 8
sleep 2

#######################################
# GIT COMMANDS
#######################################

section "7. rtk git status"
think
type_cmd "rtk git status"
rtk git status
sleep 1.5

section "8. rtk git log"
think
type_cmd "rtk git log -n 5"
rtk git log -n 5
sleep 2

section "9. rtk git diff"
think
type_cmd "rtk git diff"
rtk git diff 2>&1 | head -10 || echo "(no changes)"
sleep 1.5

section "10. rtk git add/commit/push/pull"
echo -e "${GRAY}# Minimal output: just 'ok ✓' on success${RESET}"
sleep 1
type_cmd "echo 'rtk git add → ok ✓'"
echo "rtk git add → ok ✓"
type_cmd "echo 'rtk git commit -m \"msg\" → ok ✓ abc1234'"
echo "rtk git commit -m \"msg\" → ok ✓ abc1234"
type_cmd "echo 'rtk git push → ok ✓ main'"
echo "rtk git push → ok ✓ main"
type_cmd "echo 'rtk git pull → ok ✓ 3 files +10 -2'"
echo "rtk git pull → ok ✓ 3 files +10 -2"
sleep 2

#######################################
# COMMAND EXECUTION
#######################################

section "11. rtk err - Errors only"
think
type_cmd "rtk err cargo build 2>&1 | head -5"
echo -e "${GRAY}(filters stdout, shows only errors/warnings)${RESET}"
sleep 1.5

section "12. rtk test - Test failures only"
think
type_cmd "rtk test cargo test 2>&1 | head -5"
echo -e "${GRAY}(runs tests, shows only failures → -90% tokens)${RESET}"
sleep 1.5

section "13. rtk summary - Heuristic summary"
think
type_cmd "rtk summary ls -la"
rtk summary ls -la 2>&1 | head -8
sleep 2

section "14. rtk log - Deduplicated logs"
think
type_cmd "echo 'Deduplicates repeated log lines with counts'"
echo -e "${GRAY}ERROR: Connection failed${RESET}"
echo -e "${GRAY}ERROR: Connection failed ×15${RESET}"
echo -e "${GRAY}(instead of 15 identical lines)${RESET}"
sleep 2

#######################################
# DATA COMMANDS
#######################################

section "15. rtk json - JSON structure"
think
type_cmd "rtk json package.json -d 2"
echo -e "${GRAY}(shows structure without values)${RESET}"
echo "{"
echo "  name: string,"
echo "  version: string,"
echo "  dependencies: { ... 12 keys }"
echo "}"
sleep 2

section "16. rtk deps - Dependencies"
think
type_cmd "rtk deps"
rtk deps
sleep 2

section "17. rtk env - Environment variables"
think
type_cmd "rtk env -f PATH"
rtk env -f PATH 2>&1 | head -5
sleep 2

#######################################
# CONTAINERS
#######################################

section "18. rtk docker ps"
think
type_cmd "rtk docker ps"
rtk docker ps 2>&1 | head -10 || echo "(docker not running)"
sleep 1.5

section "19. rtk docker images"
think
type_cmd "rtk docker images"
rtk docker images 2>&1 | head -10 || echo "(docker not running)"
sleep 1.5

section "20. rtk kubectl pods"
think
type_cmd "rtk kubectl pods"
rtk kubectl pods 2>&1 | head -10
sleep 1.5

section "21. rtk kubectl services"
think
type_cmd "rtk kubectl services"
rtk kubectl services 2>&1 | head -10
sleep 2

#######################################
# INIT
#######################################

section "Setup: rtk init"
think
type_cmd "rtk init --show"
rtk init --show
sleep 2

#######################################
# SUMMARY
#######################################

echo ""
echo -e "${BOLD}${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
echo -e "${BOLD}${GREEN}✓ 21 commands | 60-90% token savings${RESET}"
echo -e "${BOLD}${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
echo ""
echo -e "  ${YELLOW}cargo install rtk${RESET}"
echo -e "  ${YELLOW}rtk init --global${RESET}"
echo ""
sleep 3
