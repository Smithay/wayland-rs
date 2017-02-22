#!/usr/bin/python

import os
from travis_cargo import run_output, run, run_filter

class Args:
    branch = 'master'

def main():
    version = os.environ['TRAVIS_RUST_VERSION']
    if not 'nightly' in version:
        # Only nightly cargo supports workspaces
        print("Not uploading not-nightly docs")
        return
    repo = os.environ.get('APPVEYOR_REPO_NAME') or os.environ['TRAVIS_REPO_SLUG']
    branch = os.environ.get('APPVEYOR_REPO_BRANCH') or os.environ['TRAVIS_BRANCH']
    pr = os.environ.get('TRAVIS_PULL_REQUEST', 'false')

    if branch == 'master' and pr == 'false':
        token = os.environ['GH_TOKEN']
        commit = run_output('git', 'rev-parse', '--short', 'HEAD').strip()
        msg = 'Documentation for %s@%s' % (repo, commit)

        print('generating book...')

        print('uploading docs...')
        sys.stdout.flush()
        run('git', 'clone', 'https://github.com/davisp/ghp-import')
        run(sys.executable, './ghp-import/ghp_import.py', '-n', '-m', msg, 'target/doc')
        run_filter(token, 'git', 'push', '-fq', 'https://%s@github.com/%s.git' % (token, repo), 'gh-pages')

if __name__ == "__main__":
    main()
