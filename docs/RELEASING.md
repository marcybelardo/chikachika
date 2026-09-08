# Releasing Chikachika

Chikachika releases are published from existing annotated Git tags by the
manually triggered `Publish release` GitHub Actions workflow. The workflow
checks the tagged source on Linux and macOS, verifies that the tag matches the
Cargo package version, and publishes a GitHub Release only after every check
passes.

GitHub generates the release notes from merged pull requests. Categories are
configured in `.github/release.yml`; pull request labels determine whether an
entry appears under Features, Fixes, Documentation, or Other changes.

## Prepare the release

1. Merge all release content and release-process changes through a reviewed
   pull request. Do not include work intended for the next version.
2. Confirm that `Cargo.toml` contains the version being released and that CI is
   green on the release commit.
3. From an up-to-date checkout, create and push an annotated tag that matches
   the Cargo version:

   ```sh
   git tag -a v0.0.1 <release-commit> -m "Chikachika 0.0.1"
   git push origin v0.0.1
   ```

The tag is the frozen source snapshot. Never move, replace, or reuse a
published release tag. For `0.0.1`, use the merge commit that adds this release
workflow without including `0.0.2` implementation work.

## Publish the release

1. Open the repository's **Actions** tab and select **Publish release**.
2. Choose **Run workflow**, enter the existing tag, and choose whether GitHub
   should mark the release as a prerelease. `0.0.1` should remain a prerelease.
3. Wait for both platform checks and the documentation/browser checks. The
   publish job runs only after they pass.
4. Verify the resulting title, generated notes, source archives, and tag link
   on the Releases page.

The workflow deliberately refuses lightweight or malformed tags, a tag that
does not match `Cargo.toml`, and a tag that already has a GitHub Release. It
does not build installers or binary bundles; the current release remains a
source-based development build.
