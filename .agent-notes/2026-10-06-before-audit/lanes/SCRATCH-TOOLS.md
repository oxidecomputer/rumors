<!-- CAVEAT LECTOR: written by the coordinator (Claude Opus 5.5). -->

# The lane auditors' scratch tooling

Each lane's `scratch-tools/` directory is a copy of what that lane's auditor
built in the session scratchpad (`/private/tmp/...`), which does not survive
a reboot: scripts, mutant schemas and injection tables, census and
simulation code, diffs and patches, survivor and kill lists, and logs under
100 KB. Build directories and large logs were left behind.

These are working files, unaudited and not documentation of record. They
are kept so the instrument rescue (`instrument-rescue/`) can point at
instruments that exist nowhere else, such as the adequacy lane's wasm32
injection table and the suanpan lane's mutant schema. Paths inside them
refer to the scratchpad layout they were written in.
