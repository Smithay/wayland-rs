#!/bin/bash
set -e
DOC_BRANCH="gh-pages"

[[ "$(git symbolic-ref --short HEAD)" == "master" ]] || exit 0

msg() {
    echo "[1;34m> [1;32m$@[0m"
}

dir="$(pwd)"
last_rev="$(git rev-parse HEAD)"
last_msg="$(git log -1 --pretty=%B)"

msg "Cloning into a temporary directory..."
# The second call is to support OSX.
tmp="$(mktemp -d 2>/dev/null || mktemp -d -t 'tmp-rust-docs')"
trap "cd \"$dir\"; rm -rf \"$tmp\"" EXIT
git clone -qb master "$dir" "$tmp"

cd "$tmp"
ln -s "$dir/target" "$tmp/target"

msg "Generating documentation..."
cargo doc --no-deps

# Switch to pages
msg "Replacing documentation..."
if ! git checkout -q "$DOC_BRANCH" 2>/dev/null; then
    git checkout -q --orphan "$DOC_BRANCH"
    git rm -q --ignore-unmatch -rf .
    cat > .gitignore <<EOF
/target/
/Cargo.lock
EOF
    git add .gitignore
    git commit -m "Initial commit."
fi

# Clean and replace
git rm -q --ignore-unmatch -rf .
git reset -q -- .gitignore
git checkout -q -- .gitignore
cp -a target/doc/* .
rm target
git add .
git commit -m "Update docs for $last_rev" -m "$last_msg"
git push -qu origin "$DOC_BRANCH"
msg "Done."
