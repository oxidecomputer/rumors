#!/bin/bash
# Print "<test> :: <MEASURED line>" for every MEASURED line, sorted.
awk '/STDERR:|STDOUT:/ {hdr=$0; sub(/.*(STDERR|STDOUT): */,"",hdr); sub(/ *---.*$/,"",hdr)} /MEASURED/ {line=$0; sub(/^ */,"",line); print hdr " :: " line}' "$1" | sort
