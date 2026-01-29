#!/bin/bash
# Session Start Hook - Inject code-recall reminder

cat << 'EOF'
[MEMORY SYSTEM ACTIVE]
Call mcp__code-recall__get_briefing at the start of this session to load context from previous sessions.
After making important decisions, use mcp__code-recall__store_observation to save them.
Before major changes, use mcp__code-recall__search_memory and mcp__code-recall__check_rules.
EOF

exit 0
