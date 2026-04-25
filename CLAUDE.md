# Prompt Quiver: Development Rules

## 📋 Progress Tracking
- **Always update `PROGRESS.md`**: After every feature implementation, major refactor, or significant milestone, update the roadmap and status in `PROGRESS.md`.
- **Follow the Roadmap**: Use `PROGRESS.md` as the primary guide for the current development sequence.

## 🛠️ Build & Test Commands
- **Build**: `cargo build`
- **Test**: `cargo test`
- **Lint**: `cargo clippy`
- **Format**: `cargo fmt`

## 🏗️ Architecture
- Adhere strictly to **Clean Architecture** and the **Modular Monolith** pattern.
- Logic flows from `ui` and `app` towards `contracts`.
- Infrastructure (`infra`) implements traits defined in `contracts`.

## 🧪 Testing Strategy
- **E2E First**: Prioritize integration tests using `ratatui::backend::TestBackend`.
- **Mocking**: Use `InMemory` infrastructure mocks for deterministic testing.
