# Python adapter

`adapters/python/libforge-python` is meant to host a Python package that downloads LibForge releases, but it currently only contains an empty `install.py`/`pyproject.toml`. There is no published Python adapter yet. Once implemented it should read `libforge.yaml`, verify the signed manifest, download the correct platform archive, and expose the compiled library to Python consumers without rebuilding the Rust crate.
