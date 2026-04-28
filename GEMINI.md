# Gemini Instructions: Prompt Quiver (Rust/Ratatui Re-implementation)

Welcome, agent. Your task is to re-implement the **Prompt Quiver** application—a TUI-based staging area for AI prompts—using **Rust** and the **Ratatui** framework. 

This project follows a strict **Clean Architecture** and **Modular Monolith** approach. You are expected to deliver a high-performance, robust, and deterministically tested terminal application.

---

## 1. Documentation Map

The following specifications are available in the `docs/` directory. Review them as needed based on the context of your task to ensure alignment with project standards:

1.  **[FUNCTIONAL_SPEC.md](./docs/FUNCTIONAL_SPEC.md)**: The core logic, data model (TOML), and algorithms (staging, snippets, comments).
2.  **[UI_SPEC.md](./docs/UI_SPEC.md)**: The visual design, color palette, and component behaviors.
3.  **[TECHNICAL_DIRECTIVES.md](./docs/TECHNICAL_DIRECTIVES.md)**: The architectural rules, crate choices, and the **E2E-First** testing philosophy.
4.  **[TEST_SCENARIOS.md](./docs/TEST_SCENARIOS.md)**: The specific user journeys you must verify via integration tests.
5.  **[DEVELOPMENT_GUIDE.md](./docs/DEVELOPMENT_GUIDE.md)**: Practical guidance on terminal lifecycle, logging, and Git integration.
6.  **[WIDGETS.md](./docs/WIDGETS.md)**: Catalog of core and community Ratatui widgets for TUI development.

---

## 2. Core Mandates

- **Language:** Rust (Stable).
- **Architecture:** Cargo Workspace with hard boundaries between modules. Use the "Contracts" pattern (Traits) for dependency inversion.
- **Testing:** Prioritize End-to-End (E2E) tests using `ratatui::backend::TestBackend`. Unit tests are only for isolated, complex logic.
- **Performance:** All I/O (File system, Git polling) must be non-blocking and handled via `tokio` channels.
- **Data Safety:** Use atomic writes (temp file + rename) for all persistence.

---

## 3. Implementation Roadmap & Development Cycle

You must operate in a strict **Test-Implement-Refactor** cycle for every feature and bug fix:

**CRITICAL MANDATE (STRICT TDD COMPLIANCE):**
You **MUST** follow a strict **Red-Green-Refactor** cycle. Writing a "passing" test to document existing behavior is a violation of this mandate. The test must define the **intended** behavior and **fail** before any implementation code is touched.

1.  **Red Phase:**
    - Define the **target state** or **fix** in a new test case.
    - Run the test and **verify it fails**.
    - Do NOT write tests that confirm unwanted current behavior.
2.  **Green Phase:**
    - Write the minimal code required to make the test pass.
    - Verify the test now passes.
3.  **Refactor Phase:**
    - Clean up the implementation.
    - Ensure compliance with architectural patterns.
    - Verify all tests still pass.

**Pitfall Avoidance:**
- Never "confirm" a bug with a passing test; expose it with a failing one.
- Never modify application logic before a failing test is established.
- If you find yourself writing implementation code before a test, stop, revert, and start with the test.

### Suggested Roadmap:
1.  **Workspace Setup:** Create the Cargo Workspace and the `contracts` crate. Define your core Entities and Traits.
2.  **Infrastructure Mocks:** Implement `InMemoryStorage` and `MockClipboard` traits to enable E2E testing.
3.  **The Main Loop:** Setup the `app` crate with `crossterm`, the Alternate Screen, and a basic event loop.
4.  **Feature Modules:** Implement modules (Prompt Management, Editor, etc.) one by one using the development cycle above.
5.  **Persistence:** Implement the real TOML-based persistence in the `infra` layer.
6.  **Polishing:** Implement the "Atlas" branding, notifications (Toasts), and the Git branch background poller.

---

## 4. Operational Context

- **User Profile:** The user is an experienced software developer but is new to Rust, its ecosystem, and Ratatui. Provide clear explanations for Rust-specific idioms and crate choices when appropriate.
- **Shell:** You are working in a Windows PowerShell environment. Use `;` instead of `&&` for command chaining.
- Use `tail -f debug.log` in your mind (and suggest it to the user) to monitor your file-based logging.
- When implementating UI, refer to `./docs/UI_SPEC.md` for specific color constants (Yellow for prompts, Cyan for notes, etc.).
- **Installation:** When the user says 'install', 'install locally', or similar, you MUST run the `.\install.ps1` script for them.
