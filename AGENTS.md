# AI Agents and human operators MUST follow the rules below

1. You  MUST read the design documentation for a service before you start working on a task in that service. Design documents are located in docs/services. For instance, filemonitor's design document is located in docs/services/filemonitor.md.

2. You MUST ALWAYS update the appropriate README and / or the appropriate design document when you make a change to a service and the change is impacting what is stated in these documents. If in doubt, re-read the docs to evaluate impact.

3. You MUST use `cargo run` when you start any service for testing.

4. You MUST ALWAYS run `cargo build --all --quiet --color never`, `cargo test --all --quiet --color never` and `cargo fmt` to build the package before committing your work and fix all errors and warnings from the change you've made.

5. You MUST NEVER commit to the main branch of the git repository/ ALL work MUST happen on a feature branch.

6. You MUST commit changes summarizing all the changes since the last commit. For the author of the commit, use the configured username in git with ' (Kiro CLI)' appended and the user email. For example, `git commit --author="John Doe (Kiro CLI) <john@email.com>"`

7. When working on unit tests, you SHOULD prefer tests that will fail with clear errors (e.g. use `result.unwrap()`, instead of `assert!(result.is_ok())`).

8. You SHOULD use test that test the smallest amount of functionality possible, while still being comprehensive in aggregate.

9. You MUST use `debug!()` log messages throughout. Only use `info!()` log messages where users will derive clear advantage from them when using the services, such as `Service started succesfully`.

10. You MUST add dependencies to the workspace Cargo.toml when more than one service has the same dependency.

