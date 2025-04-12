#!/usr/bin/env bash

set -euo pipefail

# Output file
output_file="combined.rs"

# Clear the output file if it exists
> "$output_file"

# Use fd to find all .rs files (recursively by default)
fd -e rs --exclude "$output_file" | while read -r file; do
    # Add filename comment (with relative path)
    echo "//$file" >> "$output_file"
    
    # Append file content
    cat "$file" >> "$output_file"
    
    # Add a newline separator between files
    echo "" >> "$output_file"
done

echo "Combined all .rs files into $output_file"
