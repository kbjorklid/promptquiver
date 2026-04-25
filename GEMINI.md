# Gemini Instructions: Prompt Quiver (Rust/Ratatui Re-implementation)

Welcome, agent. Your task is to re-implement the **Prompt Quiver** application—a TUI-based staging area for AI prompts—using **Rust** and the **Ratatui** framework. 

This project follows a strict **Clean Architecture** and **Modular Monolith** approach. You are expected to deliver a high-performance, robust, and deterministically tested terminal application.

---

## 1. Documentation Map

Before writing any code, you MUST review the following specifications in order:

1.  **[FUNCTIONAL_SPEC.md](./FUNCTIONAL_SPEC.md)**: The core logic, data model (TOML), and algorithms (staging, snippets, comments).
2.  **[UI_SPEC.md](./UI_SPEC.md)**: The visual design, color palette, and component behaviors.
3.  **[TECHNICAL_DIRECTIVES.md](./TECHNICAL_DIRECTIVES.md)**: The architectural rules, crate choices, and the **E2E-First** testing philosophy.
4.  **[TEST_SCENARIOS.md](./TEST_SCENARIOS.md)**: The specific user journeys you must verify via integration tests.
5.  **[DEVELOPMENT_GUIDE.md](./DEVELOPMENT_GUIDE.md)**: Practical guidance on terminal lifecycle, logging, and Git integration.

---

## 2. Core Mandates

- **Language:** Rust (Stable).
- **Architecture:** Cargo Workspace with hard boundaries between modules. Use the "Contracts" pattern (Traits) for dependency inversion.
- **Testing:** Prioritize End-to-End (E2E) tests using `ratatui::backend::TestBackend`. Unit tests are only for isolated, complex logic.
- **Performance:** All I/O (File system, Git polling) must be non-blocking and handled via `tokio` channels.
- **Data Safety:** Use atomic writes (temp file + rename) for all persistence.

---

## 3. Implementation Roadmap & Development Cycle

You must operate in a strict **Test-Implement-Refactor** cycle for every feature:

1.  **Plan:** Research the requirement and define the E2E test scenario.
2.  **Test:** Write a failing E2E test in the relevant module using `TestBackend`.
3.  **Implement:** Write the minimal code to pass the test.
4.  **Refactor:** Perform a mandatory refactoring round to ensure the code meets Clean Architecture standards and Rust idioms. **Do not skip this step.**
5.  **Verify:** Run the full E2E suite to ensure no regressions.

### Suggested Roadmap:
1.  **Workspace Setup:** Create the Cargo Workspace and the `contracts` crate. Define your core Entities and Traits.
2.  **Infrastructure Mocks:** Implement `InMemoryStorage` and `MockClipboard` traits to enable E2E testing.
3.  **The Main Loop:** Setup the `app` crate with `crossterm`, the Alternate Screen, and a basic event loop.
4.  **Feature Modules:** Implement modules (Prompt Management, Editor, etc.) one by one using the development cycle above.
5.  **Persistence:** Implement the real TOML-based persistence in the `infra` layer.
6.  **Polishing:** Implement the "Atlas" branding, notifications (Toasts), and the Git branch background poller.

---

## 4. Operational Context

- **Shell:** You are working in a Windows PowerShell environment. Use `;` instead of `&&` for command chaining.
- Use `tail -f debug.log` in your mind (and suggest it to the user) to monitor your file-based logging.
- When implementating UI, refer to `UI_SPEC.md` for specific color constants (Yellow for prompts, Cyan for notes, etc.).
