#!/usr/bin/python

import os
from travis_cargo import doc_upload, Manifest

class Args:
    branch = 'master'

def main():
    version = os.environ['TRAVIS_RUST_VERSION']
    if 'beta' in version or 'nightly' in version:
       print("Not uploading not-stable docs")
       return
    manifest = Manifest('./wayland-client/', version)
    doc_upload(version, manifest, Args())

if __name__ == "__main__":
    main()
