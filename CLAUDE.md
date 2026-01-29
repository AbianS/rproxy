## Memory Management (code-recall MCP)

IMPORTANT: Use the code-recall MCP to maintain persistent memory across sessions.

### Session Start
At the START of every session, call `get_briefing` to load context:
```
mcp__code-recall__get_briefing({ focus_areas: ["current task keywords"] })
```

### Before Major Changes
Before implementing architectural decisions or significant changes:
1. Search for past decisions: `mcp__code-recall__search_memory({ query: "relevant topic" })`
2. Check applicable rules: `mcp__code-recall__check_rules({ action: "what you're about to do" })`

### After Important Decisions
Store decisions, patterns, warnings, and learnings:
```
mcp__code-recall__store_observation({
  category: "decision" | "pattern" | "warning" | "learning",
  content: "What was decided",
  rationale: "Why this approach was chosen",
  tags: ["relevant", "tags"]
})
```

### After Implementation
Record outcomes to help future sessions:
```
mcp__code-recall__record_outcome({
  memory_id: <id from store_observation>,
  outcome: "Result description",
  worked: true | false
})
```

**Categories:**
- `decision` - Architectural or implementation choices
- `pattern` - Reusable patterns that work well
- `warning` - Things to avoid or be careful about
- `learning` - Insights gained from implementation