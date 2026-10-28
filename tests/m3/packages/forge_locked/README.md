# `forge_locked` validation fixture

This fixture is a self-contained copy of `mods/forge` under package id `forge_locked`, so both
packages can be present in the same package set without duplicate content identifiers. It adds
`precision_shells`, which requires `forge_locked:hardened_shells`. Its costs are empty because
this fixture checks prerequisite resolution only.
