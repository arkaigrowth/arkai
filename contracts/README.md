# Arkai Agent Contracts

## Purpose

This directory contains **JSON Schema contracts** that define the interfaces between Arkai's multi-agent system components. Contracts ensure type-safe, validated communication across language boundaries (Rust, Python, TypeScript) and make agent responsibilities explicit.

## Philosophy

**Contracts are the source of truth for inter-agent communication.**

- **Explicit over Implicit**: All data structures exchanged between agents must have schemas
- **Language Agnostic**: Schemas work across Rust, Python, TypeScript, and any future languages
- **Versioned**: Schemas have version numbers to enable backward compatibility
- **Validated**: Agents should validate inputs/outputs against schemas
- **Self-Documenting**: Rich descriptions and examples make schemas understandable

## When to Use Contracts

### USE contracts for:
- **Agent-to-Agent Communication**: Data passed between different agents (e.g., Rust ingestion → Claudia agent)
- **Event Streams**: Events in append-only logs (e.g., voice queue events)
- **Shared State**: Data structures multiple agents read/write (e.g., queue items)
- **API Boundaries**: External inputs/outputs that cross system boundaries

### DON'T use contracts for:
- **Internal Implementation**: Private data structures within a single agent
- **Temporary Data**: Ephemeral objects that never leave a function
- **Tightly Coupled Code**: Code in the same file/module that shares types directly

## Contract Structure

Each schema should follow this pattern:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://arkai.dev/schemas/<name>.schema.json",
  "title": "Human-Readable Title",
  "description": "What this contract defines and why it exists",
  "version": "1.0.0",

  "definitions": {
    "MainType": {
      "type": "object",
      "description": "Clear description of purpose",
      "required": ["field1", "field2"],
      "properties": {
        "field1": {
          "type": "string",
          "description": "What this field represents and when it's set",
          "examples": ["example_value"]
        }
      }
    }
  }
}
```

### Required Elements:
- **$schema**: Always use draft-07
- **$id**: Unique identifier for this schema
- **title**: Human-readable name
- **description**: What problem this contract solves
- **version**: Semantic version (MAJOR.MINOR.PATCH)
- **examples**: Real-world example values for every complex type

## How Agents Should Use Contracts

### 1. Read the Contract First
Before implementing agent communication, read the schema to understand:
- What data is required vs optional
- What formats are expected (ISO 8601 dates, specific enums, etc.)
- What examples look like

### 2. Validate Inputs
When receiving data from another agent:

**Python Example:**
```python
import jsonschema
import json

with open("contracts/voice_intake.schema.json") as f:
    schema = json.load(f)

# Validate incoming data
jsonschema.validate(instance=queue_item, schema=schema["definitions"]["VoiceQueueItem"])
```

**Rust Example:**
```rust
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
struct VoiceQueueItem {
    item_id: String,
    status: QueueStatus,
    // ... fields from schema
}
```

**TypeScript Example:**
```typescript
import Ajv from "ajv";
import schema from "./contracts/voice_intake.schema.json";

const ajv = new Ajv();
const validate = ajv.compile(schema.definitions.VoiceQueueItem);

if (!validate(data)) {
  console.error("Validation errors:", validate.errors);
}
```

### 3. Update Contracts When Evolving
When adding new fields or changing behavior:

1. **Update the schema first** (in `contracts/`)
2. **Increment the version** (MAJOR for breaking changes, MINOR for new fields)
3. **Update all affected agents** to match the new contract
4. **Add migration notes** if backward compatibility is broken

## Existing Contracts

### `voice_intake.schema.json`
**Purpose**: Communication between voice ingestion system (Rust) and Claudia agent (Python)

**Key Types**:
- `VoiceQueueItem`: Current state of a queued voice memo
- `VoiceEvent`: Immutable events in the voice queue log
- `IntentClassification`: Claudia's analysis of voice memo intent

**Agents Using This**:
- `arkai-daemon` (Rust): Writes queue items, reads status
- `claudia` (Python/Claude): Reads pending items, writes classifications

**Example Flow**:
1. Rust daemon detects audio file → writes `enqueued` event
2. Claudia reads pending item → transcribes → classifies intent
3. Claudia creates Obsidian note → writes `completed` event with paths

## Best Practices

### Naming Conventions
- **Files**: `snake_case.schema.json` (e.g., `voice_intake.schema.json`)
- **Types**: `PascalCase` (e.g., `VoiceQueueItem`)
- **Fields**: `snake_case` (e.g., `item_id`, `detected_at`)
- **Enums**: `SCREAMING_SNAKE_CASE` for values (e.g., `"TASK"`, `"NOTE"`)

### Description Quality
Good descriptions answer:
- **What**: What does this field represent?
- **When**: When is it set/updated?
- **Who**: Which agent is responsible for setting it?
- **Format**: What format/constraints apply?

**Good Example**:
```json
"detected_at": {
  "type": "string",
  "format": "date-time",
  "description": "ISO 8601 timestamp when file was first detected by the ingestion system"
}
```

**Bad Example**:
```json
"detected_at": {
  "type": "string",
  "description": "Timestamp"
}
```

### Evolution Strategy
- **Additive Changes**: Add new optional fields (MINOR version bump)
- **Breaking Changes**: Remove fields, change types, change semantics (MAJOR version bump)
- **Deprecation**: Mark fields as deprecated in description before removing
- **Migration**: Provide migration scripts for breaking changes

### Testing
Add contract validation tests to your agent tests:

```python
def test_queue_item_schema_compliance():
    """Ensure our queue items match the contract"""
    item = create_test_queue_item()
    validate_against_schema(item, "voice_intake.schema.json", "VoiceQueueItem")
```

## Contract Review Checklist

Before committing a new/updated contract:

- [ ] All required fields are documented
- [ ] All fields have clear descriptions (what, when, who, format)
- [ ] Examples are provided for complex types
- [ ] Enum values are explicitly listed
- [ ] Version number is correct (follows semantic versioning)
- [ ] Backward compatibility is considered
- [ ] Affected agents are identified in documentation
- [ ] Types use standard formats (ISO 8601 for dates, etc.)

## Resources

- [JSON Schema Specification (Draft-07)](https://json-schema.org/draft-07/schema)
- [Understanding JSON Schema](https://json-schema.org/understanding-json-schema/)
- [JSON Schema Validator](https://www.jsonschemavalidator.net/)

## Questions?

If unsure whether something needs a contract, ask:
1. Does this data cross an agent boundary?
2. Would multiple agents benefit from knowing this structure?
3. Could validation prevent bugs?

If yes to any, create a contract.
