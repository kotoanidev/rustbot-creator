# Releasing

How RustBot Creator is packaged and released on Linux (D-144–D-150, C-036). Fedora gets an
RPM from Copr, and Ubuntu gets a .deb from GitHub Releases and a signed apt repository on
GitHub Pages. A `vX.Y.Z` tag releases everything.

## The files

- `packaging/linux/`: the desktop file, the AppStream metainfo and the icon, all named
  after the app ID `io.github.kotoanidev.RustBotCreator` (D-147).
- `packaging/rpm/rustbot-creator.spec` and `.copr/Makefile`: the RPM, and Copr's
  "make srpm" source method (D-144).
- `packaging/deb/build-deb.sh`: the .deb, made from a release build (D-145).
- `packaging/apt/update-repo.sh`: adds .deb files to the apt repository and signs it.
- `packaging/third-party-licenses.py`: the license notices of every crate built into the
  program, shipped in both packages.
- `packaging/check-version.sh`: checks that a tag matches every version number.
- `packaging/release-notes.md`: the release's notes, with the install instructions.
- `.github/workflows/release.yml`: run by hand, it builds and checks both packages and
  publishes nothing. For a tag, it also creates the GitHub release and updates the apt
  repository.

## One-time setup (the owner)

1. **Copr** (needs a Fedora account):
   1. Create the project `rustbot-creator` at <https://copr.fedorainfracloud.org>, with
      the chroots `fedora-43-x86_64` and `fedora-44-x86_64`. In its settings, switch on
      "Enable internet access during builds".
   2. Add the package `rustbot-creator`:
      - source type: SCM;
      - clone URL: `https://github.com/kotoanidev/rustbot-creator.git`;
      - spec file: `packaging/rpm/rustbot-creator.spec`;
      - SRPM build method: "make srpm";
      - "Auto-rebuild": on.
   3. Under "Integrations", copy the GitHub webhook URL. In the repository's
      Settings → Webhooks, add it with content type `application/json`. Pick "Let me
      select individual events", then only "Branch or tag creation", so Copr builds tags
      and not every push. On the first tag, check that Copr starts a build.
2. **The apt signing key.** Make a key used only for this, without a passphrase, since
   GitHub keeps it secret:

   ```sh
   gpg --batch --passphrase '' --quick-gen-key "RustBot Creator apt repository" rsa4096 sign never
   gpg --armor --export-secret-keys "RustBot Creator apt repository"
   ```

   Paste the second command's output into a new repository secret `APT_GPG_KEY`
   (Settings → Secrets and variables → Actions). Keep a copy offline: without it, every
   user would have to import a new key.
3. **GitHub Pages**, after the first release has made the `gh-pages` branch: in
   Settings → Pages, deploy from the branch `gh-pages`, folder `/`.

## Releasing a version

1. Release only once the owner has tested the app live and agrees (D-150).
2. Set the new version in four places:
   - `Cargo.toml` (`[workspace.package]`);
   - the spec's `Version`, with `Release: 1` and a new `%changelog` entry;
   - the metainfo's `<release>`, with today's date;
   - the date and notes in `packaging/release-notes.md`, if they changed.
   Then `packaging/check-version.sh vX.Y.Z` must print the version.
3. Run the Release workflow by hand on `main` (Actions → Release → Run workflow). Both
   packages must build, install and start.
4. Tag and push:

   ```sh
   git tag -a vX.Y.Z -m "RustBot Creator X.Y.Z"
   git push origin vX.Y.Z
   ```

5. Watch the workflow publish the .deb, the release and the apt repository, and Copr
   build the RPM.
6. Install from Copr and from apt on clean Fedora and Ubuntu systems (e.g. in a VM), and
   start the app.
7. For the first release only, add the install instructions of
   `packaging/release-notes.md` to the README.
