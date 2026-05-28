# Built-in Modules Source Layout

This directory contains built-in module definitions used by Mire.

Layout rule:
- `src/modules/<module_name>/...` for top-level modules
- `src/modules/std/<section>/mod.mire` for Standard Library sections

Current migration:
- `std` split into section files under `src/modules/std/`
- `std` aggregator moved to `src/modules/std/mod.mire`
