#!/usr/bin/python

import os
from travis_cargo import doc_upload, Manifest

class Args:
    branch = 'master'

def main():
    version = os.environ['TRAVIS_RUST_VERSION']
    if not 'nightly' in version:
        # Only nightly cargo supports workspaces
        print("Not uploading not-nightly docs")
        return
    manifest = Manifest('./wayland-client/', version)
    doc_upload(version, manifest, Args())

if __name__ == "__main__":
    main()
