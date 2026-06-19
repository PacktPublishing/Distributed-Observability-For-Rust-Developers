# Agent Instructions

## Core Principles

### Do Only What Is Asked
- **Strict Scope Adherence**: Perform only the specific tasks requested by the user
- **No Additional Work**: Do not add features, optimizations, or enhancements unless explicitly asked
- **No Unsolicited Documentation**: Do not create summary documents, reports, or additional documentation unless specifically requested
- **Focus on the Task**: Complete the exact request without expanding the scope

### Documentation Guidelines
- Only create documentation when explicitly asked to do so
- Do not write summary documents, progress reports, or status updates unless requested
- If documentation is requested, ask for clarification on the specific type and scope needed
- Avoid creating README files, changelogs, or other documentation as "helpful additions"

### Response Guidelines
- Provide concise confirmations when tasks are complete
- Ask clarifying questions if the request is ambiguous
- Do not offer suggestions for additional work unless asked
- Keep responses focused on the specific request

### Code Quality Guidelines
- **Always Include Comments**: When writing code, include clear and meaningful comments
- Explain complex logic, business rules, and non-obvious implementations
- Use comments to describe the purpose and intent of functions, classes, and modules
- Add inline comments for tricky or performance-critical sections

### Code Organization Guidelines
- **API Response Structs**: If a `struct` is used for API response, it must be created/moved to the models folder
- Keep response models separate from internal domain models
- Ensure consistent structure across API responses

## Implementation Rules
1. Read the request carefully and identify the exact scope
2. Complete only what was explicitly asked for
3. Confirm completion without suggesting next steps
4. Do not create supporting documentation unless requested
