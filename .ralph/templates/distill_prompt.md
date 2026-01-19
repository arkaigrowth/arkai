# RALPH Distillation Prompt

> This prompt is used to generate session artifacts from a transcript.
> Run at session end via: `ralph close`

---

## Input

You will receive:
1. **Transcript**: Full session conversation
2. **Tool Log**: All tool calls made (JSONL)
3. **Repo Diff**: Git changes made during session

---

## Task

Analyze the session and produce exactly 5 artifacts:

### 1. summary.md

Write a narrative summary (200-500 words) covering:
- What was the goal?
- What was accomplished?
- What approaches were tried?
- What worked vs. didn't work?

Format: Prose paragraphs, past tense, third person.

### 2. decisions.md

Extract every explicit decision made. For each:

```markdown
## [Decision Title]

**Choice**: What was decided
**Alternatives Considered**: What else was possible
**Rationale**: Why this choice
**Reversibility**: Easy / Moderate / Hard to undo
```

### 3. open_questions.md

List unresolved items, prioritized:

```markdown
## Priority 1 (Blocking)
- [ ] Question/issue that blocks progress

## Priority 2 (Important)
- [ ] Should address soon

## Priority 3 (Nice to Have)
- [ ] Can defer
```

### 4. next_prompt.md

Generate a bootstrap prompt for the next session:

```markdown
# Next Session: [Brief Title]

## Context
[1-2 sentences on where we left off]

## Immediate Tasks
1. [Most urgent item]
2. [Second item]
3. [Third item]

## Constraints
[Any new constraints discovered]

## Files to Review
- path/to/relevant/file.rs
- path/to/another/file.md
```

### 5. retrieval_index.json

Create a searchable index:

```json
{
  "topics": {
    "topic_name": ["path/to/file1", "path/to/file2"],
    "another_topic": ["path/to/file3"]
  },
  "key_files": ["most/important/file.rs"],
  "changed_files": ["files/modified/this/session.rs"],
  "keywords": ["important", "search", "terms"]
}
```

---

## Quality Criteria

- **Accuracy**: Only include what actually happened
- **Completeness**: Don't omit significant decisions or blockers
- **Actionability**: Next prompt should be directly usable
- **Searchability**: Index should enable future retrieval

---

## Output Format

Return all 5 artifacts clearly delimited:

```
===== summary.md =====
[content]

===== decisions.md =====
[content]

===== open_questions.md =====
[content]

===== next_prompt.md =====
[content]

===== retrieval_index.json =====
[content]
```
