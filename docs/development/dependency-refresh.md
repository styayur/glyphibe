# Refreshing dependency evidence

Dependency PRs change the inputs bound into the committed license inventory.
After combining compatible updates, run:

```sh
npm ci --ignore-scripts --prefix apps/desktop
python scripts/audit-dependencies.py --fetch-missing-license-texts
python scripts/audit-dependencies.py --check
python scripts/audit-native-notices.py
python -m unittest discover -s tests -p 'test_*.py'
```

Review and commit the regenerated inventory, SPDX document, notices and any new
original license texts with the manifest/lock changes. Do not edit input hashes
to make an old inventory appear current. Network notice recovery verifies npm
tarball integrity and retains exact notice bytes without executing package code.

For an environment without all source packages, manually dispatch CI on the
dependency branch. Its `Generate dependency inventory candidate` job produces
an artifact with those files for review and commitment. It never writes to the
repository. The normal dependency audit still checks the committed inventory;
the candidate job cannot make a stale PR pass that check. All security audit,
platform build and test checks remain required as configured by the repository.
