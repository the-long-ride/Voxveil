# Test Strategy

## Coverage

Minimum 85% line coverage for each major domain and globally where tooling supports aggregation.

- UI/TypeScript: >= 85%.
- Rust workspace: >= 85%.
- DSP: >= 85%.
- Routing: >= 85%.
- Tauri application logic: >= 85% excluding unavoidable platform-generated glue.

## LOC Gates

Hard limits:

- `.ts`: 300 lines
- `.tsx`: 400 lines
- `.rs`: 300 lines
- `.css`: 400 lines

The repository-owned LOC checker is a CI test gate. Generated/vendored directories may be excluded only by explicit path rules.

## Test Types

- Unit tests for domain logic.
- Golden/numeric DSP tests.
- Routing invariant tests.
- Tauri boundary tests where practical.
- Responsive/i18n UI tests.
- Platform adapter contract tests with mocks/fakes when CI cannot access system audio APIs.
