# Changelog

## [0.1.1] - 2024-06-07

### Bug Fixes

- git: Ensure git diff does never use an external diff tool
- cli: Force color output in spawned pipes
- diff: Handle hunks that have no offset in the diff

### Features

- changelog: Add git cliff config to generate changelogs

### Miscellaneous Tasks

- deny: Add cargo deny configuration
- clippy: Add configuration
- make: Add one shot release target with changelog generator
- editor: Adding editor config
- cleanup: Apply clippy suggestions
- ci: Cleanup github ci matrix arch
- clippy: Refactor equatable_if_let
- deps: Switch from structopt to clap 4 #2
- deps: Update all dependencies
- deps: Update dependencies
- deps: Upgrade all dependencies
- ci: Use cargp-deny instead of audit in github ci

## [0.1.0] - 2022-10-25

### Bug Fixes

- crash: Exit if no commits were found before spawning the menu
- rebase: Override GIT_SEQUENCE_EDITOR in none interactive mode
- dedup: Use no abbrev for all commit revs

### Documentation

- Adding links to build status and release badges

### Features

- format: Add placeholder for source of target suggestion
- git: Add function to retrieve the system git version
- listing: Add mode to list (unrelated) recent history
- cli: Add opt groups to mutually exclusive options
- cli: Add option to quickly smash directly into target commit
- option: Add support for amend and reword fixups
- cli: Allow to pass a number for amount of recent commits
- dedup: Avoid dedup on log line by using no abbrev commit hashes
- ci: Build on all branches and tags
- cli: Support short opt for --interactive
- format: Support special placeholder for smash source

### Miscellaneous Tasks

- simplify: Add helper to check specific git version for features
- format: Apply cargo format
- lint: Apply nightly clippy recommendations for let equals checks
- cleanup: Avoid mut ref for simple getter
- error: Improve sub command error handling instead of exiting
- cleanup: Make linter happy and replace deprecated APIs
- opt: Rename range option name to revision-range
- cargo: Switch rust edition to 2021
- deps: Update dependencies
- deps: Upgrade all dependencies
- cleanup: Use clippy recommended else unwrapping

### Version

- Release 0.1.0

## [0.0.2] - 2021-03-11

### Bug Fixes

- Avoid invalid utf8 panic with binary line split and lossy conversion
- Avoid pager ever being called when listing git stages files

### Documentation

- Add super basic readme skeleton

### Features

- Add clap completion generator via cli option
- Add commit list based on blame chunks
- Add options and config to list blame chunks or files history
- Only list unique targets not yet shown

### Miscellaneous Tasks

- Add make targets for convenience
- Adding MIT license
- Disable wrong self convention until clippy is fixed
- Make clippy happy like a hippo
- Remove useless question mark and return Result instead

### Ci

- Adding gitlab scheduled workflow

### Version

- Release 0.0.2

## [0.0.1] - 2021-03-04

### Bug Fixes

- Abort smash if the fixup commit fails
- Change cwd to git toplevel directory for all subsequent commands
- Check for staged files before spawning the menu command
- Gracefully handle closed pipe on stdout and stderr
- Verify whether the menu command returned a valid git rev

### Features

- Add auto rebase mode to smash the fixup into the target
- Add git commit fixup
- Add interactive option to edit the final rebase draft
- Add option to limit listed revs to local non published commits
- Add option to set the number of commits per file
- Add option to specify git log format to show targets
- First iteration proof of concept
- Flatten cmd pipeline into git log with unlimited streaming
- Implement select (menu) and list (print) mode
- Implemented git config fallback for all options
- Support sk and fzf as fuzzy matcher for the menu

### Miscellaneous Tasks

- Add tiny cargo description
- Adding clippy modes to deny list
- Format the whole code base via cargo fmt
- Improve error handling instead of unwrapping without context
- Improve performance by avoiding shellout per commit
- Move all git fn to own file
- Remove unneeded wait on shell command
- Separate git file rev list command into separated function
- Simplify code as getting all staged files is virtually a no-op
- Simplify git_commit_fixup by inheriting stdout
- Tiny code cleanup for better readable ans more convenient code
- Tiny code cleanup with removed mut burrow that isn't required

### Api

- Adding builder to query git config values


