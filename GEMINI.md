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

**CRITICAL MANDATE (TDD STRICT COMPLIANCE):** 
You **MUST** write and execute a failing test **BEFORE** making any changes to the application code. If you modify implementation code before a failing test has been successfully created, run, and verified to fail, you are actively violating your core instructions. **There are no exceptions.**

1.  **Bug Reporting:** When a bug is reported, your very first action MUST be to write a failing test case that reproduces the issue. Run the test and confirm it fails before attempting any fix.
2.  **Plan:** Research the requirement or bug and define the E2E test scenario.
3.  **Test FIRST:** Write the E2E test in the relevant module using `TestBackend`. **Run the test to ensure it fails.**
4.  **Implement:** Write the minimal code to pass the test.
5.  **Refactor:** Perform a mandatory refactoring round to ensure the code meets Clean Architecture standards and Rust idioms. **Do not skip this step.**
6.  **Verify:** Run the full E2E suite to ensure no regressions.

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
