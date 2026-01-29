---
name: code-recall
description: Use semantic memory from code-recall MCP to remember decisions, patterns, and learnings across sessions. Call get_briefing at session start, check_rules before major changes, and store_observation after important decisions.
---

# code-recall Memory Integration

This skill helps you interact with the code-recall MCP server for persistent memory across coding sessions.

## Session Start

At the beginning of each coding session, call `get_briefing` to:
- Get statistics about stored memories
- See recent warnings and failed approaches
- Pre-fetch context for your focus areas

```json
{
  "tool": "get_briefing",
  "arguments": {
    "focus_areas": ["authentication", "database"]
  }
}
```

## Before Making Changes

Before implementing significant changes:

1. **Search past decisions** with `search_memory`:
```json
{
  "tool": "search_memory",
  "arguments": {
    "query": "authentication approach",
    "category": "decision"
  }
}
```

2. **Check applicable rules** with `check_rules`:
```json
{
  "tool": "check_rules",
  "arguments": {
    "action": "adding API endpoint for user authentication"
  }
}
```

## After Making Decisions

After making architectural or important decisions, store them with `store_observation`:

```json
{
  "tool": "store_observation",
  "arguments": {
    "category": "decision",
    "content": "Use JWT for authentication instead of sessions",
    "rationale": "Stateless auth enables horizontal scaling and works better with our microservices architecture",
    "tags": ["auth", "jwt", "architecture"]
  }
}
```

### Categories

- `decision` - Architectural or implementation choices
- `pattern` - Reusable patterns that work well
- `warning` - Things to avoid or be careful about
- `learning` - Insights gained from implementation

## After Implementation

Once you verify if something worked or failed, record the outcome:

```json
{
  "tool": "record_outcome",
  "arguments": {
    "memory_id": 1,
    "outcome": "JWT implementation works well, token refresh handled smoothly",
    "worked": true
  }
}
```

**Important:** Setting `worked: false` boosts that memory in future searches, helping avoid repeated mistakes.

## Creating Rules

To establish project guardrails, use `set_rule`:

```json
{
  "tool": "set_rule",
  "arguments": {
    "trigger": "adding API endpoint",
    "must_do": ["Add rate limiting", "Write integration tests", "Update API documentation"],
    "must_not": ["Skip authentication middleware"],
    "ask_first": ["Is this a breaking change?", "Does this need versioning?"]
  }
}
```

## Example Workflow

1. **Session starts**
   ```
   get_briefing({ focus_areas: ["payments"] })
   ```

2. **About to implement payments**
   ```
   search_memory({ query: "payment integration" })
   check_rules({ action: "adding payment processing" })
   ```

3. **Decide on Stripe**
   ```
   store_observation({
     category: "decision",
     content: "Use Stripe for payments",
     rationale: "Best documentation, webhook support, and PCI compliance handled"
   })
   ```

4. **After implementation works**
   ```
   record_outcome({
     memory_id: 5,
     outcome: "Stripe integration complete, webhooks working reliably",
     worked: true
   })
   ```

## When to Store Memories

**Do store:**
- Architectural decisions and their rationale
- Patterns that proved useful
- Approaches that failed (with `worked: false`)
- Project-specific rules and conventions

**Don't store:**
- Trivial implementation details
- Temporary debugging notes
- Information already in documentation
