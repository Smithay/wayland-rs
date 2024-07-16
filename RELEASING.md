Release Process
===============

Wayland-rs has a CI automation for releasing all the subcrates on crates.io when a release is
tagged. After updating the changelogs and versions.

* Create a branch for the release
* `cargo release` (install with `cargo install cargo-release`) can be used to bump versions
  - For instance, `cargo release --no-publish --no-tag --no-push --execute patch`
* Amend the commit to update the changelogs of any subcrates with changelog entries to have a release
  date.
* Create a pull request with the release. Check that there are no warnings from `publish` CI job
  - Those will become hard errors when a release is tagged
* Merge PR, and tag as `release-YYYY-MM-DD`, with the current date
* When the tag is pushed, CI will run and release to crates.io
