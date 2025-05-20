# Repository structure
- there are 2 main folders
- lib/ holds all of our code that is needed  for scores calculation and tlsn / merkel proofs verification 
- risc0_proofs/ holds risczero proofs that import and execute code from lib/ inside of ZKvm



## lib/ folder structure
- lib/ holds all of our code that is needed  for scores calculation and tlsn / merkel proofs verification 
to build it:
```bash
cd lib/

// lib folder is configured as rust workspace so on build all of the subfolder will get built
cargo build 

// to run all of the unit tests run
cargo test 
```

### 1. Core/
holds logic for merkle and tlsn proofs verification 
### 2. integration-test/
is a placeholder for integration tests. (it might be deleted later )
### 3. proofs/
for each risc0_proof there should be an sub folder in proofs/ folder that defines the logic for host and guest program. This will allow us executing this code outside of the ZKvm inside of automated tests that will make debuging process much faster and easier
### 4. test_data/
holds all of the data that is used across the repository for tests
### 5. utils/
is supposed to hold logic for credit score calculation.(we might use core folder for this instead. To be disscused)
```
├── Cargo.lock
├── Cargo.toml // workspace cargo file 
├── core
│   ├── merkle_verifier_core
│   │   ├── Cargo.toml
│   │   ├── src
│   │   │   ├── lib.rs
│   │   │   ├── main.rs
│   │   │   └── merkle_patricia.rs
│   │   └── tests
│   │       └── patricia_test.rs
│   └── tlsn-verifier-core
│       ├── Cargo.lock
│       ├── Cargo.toml
│       ├── src
│       │   ├── lib.rs
│       │   ├── main.rs
│       │   ├── types.rs
│       │   └── verification.rs
│       └── tests
│           └── verificaiton_test.rs
├── integration-test
│   ├── Cargo.lock
│   ├── Cargo.toml
│   ├── src
│   │   └── main.rs
│   └── tests
│       └── tlsn_integration_tests.rs
├── proofs
│   └── example_merkle_verify
│       ├── Cargo.toml
│       ├── src
│       │   ├── guest.rs
│       │   ├── host.rs
│       │   └── lib.rs
│       └── tests
├── test_data
│   ├── merkel-proofs
│   └── tlsn
│       └── valid_presentation.json
└── utils
```
# Git Commit Message Convention

According to [https://www.conventionalcommits.org/en/v1.0.0/#specification](https://www.conventionalcommits.org/en/v1.0.0/#specification) we can build our custom template  

  **!! Before commiting do first!!**
  1. check if cargo build / run  works
  2. check if all headless tests are passing

**Commit message template**

```
<type>(<crate_name>-<?ticket_number>): <description>
```
  

The commit <`type`> can include the following:

- `init` – a new crate is introduced. 
- `feat` – a new feature is introduced with the changes
- `fix` – a bug fix has occurred
- `chore` – changes that do not relate to a fix or feature and don't modify src or test files (for example updating dependencies)
- `refactor` – refactored code that neither fixes a bug nor adds a feature
- `docs` – updates to documentation such as a the README or other markdown files
- `style` – changes that do not affect the meaning of the code, likely related to code formatting such as white-space, missing semi-colons, and so on.
- `test` – including new or correcting previous tests
- `perf` – performance improvements
- `ci` – continuous integration related
- `build` – changes that affect the build system or external dependencies
- `revert` – reverts a previous commit

The <`ticket_number`> is optional kaban ticket number

`<description>` should follow some recommendations:

- Mood: Use imperative mood in the subject line. Example – `Add fix for dark mode toggle state`. Imperative mood gives the tone you are giving an order or request.
- Length: The first line should ideally be no longer than 50 characters

Example:

|                                           |
| ----------------------------------------- |
| `fix(tlsn-risc0-``112``): Fixed Imports ` |

