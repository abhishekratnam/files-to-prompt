# files-to-prompt

[![Crates.io](https://img.shields.io/crates/v/files-to-prompt)](https://crates.io/crates/files-to-prompt)
[![Build Status](https://github.com/yourusername/files-to-prompt/workflows/CI/badge.svg)](https://github.com/yourusername/files-to-prompt/actions)
[![License](https://img.shields.io/crates/l/files-to-prompt)](LICENSE)

Concatenate a directory full of files into a single prompt for use with LLMs.

This is a Rust implementation of the [Python files-to-prompt tool](https://github.com/simonw/files-to-prompt) by Simon Willison.

## Installation

Install using Cargo:

```bash
cargo install files-to-prompt
```

Or clone and build from source:

```bash
git clone https://github.com/yourusername/files-to-prompt.git
cd files-to-prompt
cargo build --release
```

## Usage

To use files-to-prompt, provide the path to one or more files or directories you want to process:

```bash
files-to-prompt path/to/file_or_directory [path/to/another/file_or_directory ...]
```

This will output the contents of every file, with each file preceded by its relative path and separated by `---`.

## Options

- `-e, --extension <extension>`: Only include files with the specified extension. Can be used multiple times.

  ```bash
  files-to-prompt path/to/directory -e txt -e md
  ```

- `--include-hidden`: Include files and folders starting with `.` (hidden files and directories).

  ```bash
  files-to-prompt path/to/directory --include-hidden
  ```

- `--ignore <pattern>`: Specify one or more patterns to ignore. Can be used multiple times.

  ```bash
  files-to-prompt path/to/directory --ignore "*.log" --ignore "temp*"
  ```

- `--ignore-files-only`: Include directory paths which would otherwise be ignored by an `--ignore` pattern.

  ```bash
  files-to-prompt path/to/directory --ignore-files-only --ignore "*dir*"
  ```

- `--ignore-gitignore`: Ignore .gitignore files and include all files.

  ```bash
  files-to-prompt path/to/directory --ignore-gitignore
  ```

- `-c, --cxml`: Output in Claude XML format.

  ```bash
  files-to-prompt path/to/directory --cxml
  ```

- `-m, --markdown`: Output as Markdown with fenced code blocks.

  ```bash
  files-to-prompt path/to/directory --markdown
  ```

- `-o, --output <file>`: Write the output to a file instead of printing it to stdout.

  ```bash
  files-to-prompt path/to/directory -o output.txt
  ```

- `-n, --line-numbers`: Include line numbers in the output.

  ```bash
  files-to-prompt path/to/directory -n
  ```

  Example output:
  ```
  src/main.rs
  ---
    1  use std::env;
    2  use std::process;
    3
    4  fn main() {
  ...
  ```

- `-0, --null`: Use NUL character as separator when reading paths from stdin. Useful when filenames may contain spaces.

  ```bash
  find . -name "*.rs" -print0 | files-to-prompt --null
  ```

## Example

Suppose you have a directory structure like this:

```
my_project/
├── src/
│   ├── main.rs
│   └── lib.rs
├── .hidden_file.txt
├── Cargo.toml
└── tests/
    └── test_main.rs
```

Running `files-to-prompt my_project` will output:

```
my_project/src/main.rs
---
Contents of main.rs
---
my_project/src/lib.rs
---
Contents of lib.rs
---
my_project/Cargo.toml
---
Contents of Cargo.toml
---
my_project/tests/test_main.rs
---
Contents of test_main.rs
---
```

If you run `files-to-prompt my_project --include-hidden`, the output will also include `.hidden_file.txt`.

## Reading from stdin

The tool can also read paths from standard input. This can be used to pipe in the output of another command:

```bash
# Find Rust files
find . -name "*.rs" | files-to-prompt
```

When using the `--null` (or `-0`) option, paths are expected to be NUL-separated (useful when dealing with filenames containing spaces):

```bash
find . -name "*.rs" -print0 | files-to-prompt --null
```

You can mix and match paths from command line arguments and stdin:

```bash
# Include files modified in the last day, and also include README.md
find . -mtime -1 | files-to-prompt README.md
```

## Claude XML Output

Anthropic has provided specific guidelines for optimally structuring prompts to take advantage of Claude's extended context window.

To structure the output in this way, use the optional `--cxml` flag, which will produce output like this:

```xml
<documents>
<document index="1">
<source>my_project/src/main.rs</source>
<document_content>
Contents of main.rs
</document_content>
</document>
<document index="2">
<source>my_project/src/lib.rs</source>
<document_content>
Contents of lib.rs
</document_content>
</document>
</documents>
```

## Markdown Fenced Code Block Output

The `--markdown` option will output the files as fenced code blocks, which can be useful for pasting into Markdown documents.

```bash
files-to-prompt path/to/directory --markdown
```

The language tag will be guessed based on the filename extension.

If the code itself contains triple backticks, the wrapper around it will use one additional backtick.

Example output:

```
src/main.rs
```rust
fn main() {
    println!("Hello, world!");
}
```

src/complex.rs
````rust
fn example() {
    // This code has backticks in it
    let markdown = ```
    # Title
    Some content
    ```;
}
````
```

## Development

To contribute to this tool, first checkout the code:

```bash
git clone https://github.com/yourusername/files-to-prompt.git
cd files-to-prompt
```

Build the project:

```bash
cargo build
```

Run tests:

```bash
cargo test
```

## License

Apache-2.0

## Credits

This is a Rust implementation inspired by the Python [files-to-prompt](https://github.com/simonw/files-to-prompt) tool by Simon Willison.