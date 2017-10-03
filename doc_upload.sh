#!/bin/sh

repo="$TRAVIS_REPO_SLUG"
token="$GITHUB_TOKEN"
commit=$(git rev-parse --short HEAD)
msg="Documentation for $repo@$commit"

# add a homepage
cp ./doc_index.html ./target/doc/index.html
cp ./rust.css ./target/doc/rust.css

git clone https://github.com/davisp/ghp-import
./ghp-import/ghp_import.py -n -m "$msg" target/doc
# pipe output to /dev/null to avoid printing our token in travis log
git push -fq "https://$token@github.com/$repo.git" "gh-pages" > /dev/null
