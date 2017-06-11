#!/usr/bin/python

import os
import sys
from travis_cargo import run_output, run, run_filter

class Args:
    branch = 'master'

def main():
    version = os.environ['TRAVIS_RUST_VERSION']
    if not 'stable' in version:
        print("Not uploading not-stable docs")
        return
    repo = os.environ['TRAVIS_REPO_SLUG']
    branch = os.environ['TRAVIS_BRANCH']
    pr = os.environ.get('TRAVIS_PULL_REQUEST', 'false')

    if branch == 'master' and pr == 'false':
        token = os.environ['GH_TOKEN']
        commit = run_output('git', 'rev-parse', '--short', 'HEAD').strip()
        msg = 'Documentation for %s@%s' % (repo, commit)

        print('uploading docs...')
        sys.stdout.flush()
        run('git', 'clone', 'https://github.com/davisp/ghp-import')
        run(sys.executable, './ghp-import/ghp_import.py', '-n', '-m', msg, 'target/doc')
        run_filter(token, 'git', 'push', '-fq', 'https://%s@github.com/%s.git' % (token, repo), 'gh-pages')

if __name__ == "__main__":
    main()
