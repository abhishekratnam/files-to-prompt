use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
use regex::Regex;

// Helper function to extract filenames from CXML format
fn filenames_from_cxml(cxml_string: &str) -> std::collections::HashSet<String> {
    let re = Regex::new(r"<source>(.*?)</source>").unwrap();
    re.captures_iter(cxml_string)
        .map(|cap| cap[1].to_string())
        .collect()
}

// Helper function to run CLI command and return output
fn run_cli(args: &[&str], cwd: &Path) -> std::process::Output {
    Command::new("cargo")
        .arg("run")
        .arg("--")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("Failed to execute command")
}

// Helper function to run CLI command with stdin input
fn run_cli_with_stdin(args: &[&str], cwd: &Path, stdin: &str) -> std::process::Output {
    let mut cmd = Command::new("cargo")
        .arg("run")
        .arg("--")
        .args(args)
        .current_dir(cwd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start command");

    if let Some(mut stdin_handle) = cmd.stdin.take() {
        stdin_handle.write_all(stdin.as_bytes()).expect("Failed to write to stdin");
    }

    cmd.wait_with_output().expect("Failed to read output")
}

#[test]
fn test_basic_functionality() {
    let temp_dir = TempDir::new().unwrap();
    let test_dir = temp_dir.path().join("test_dir");
    fs::create_dir(&test_dir).unwrap();
    
    fs::write(test_dir.join("file1.txt"), "Contents of file1").unwrap();
    fs::write(test_dir.join("file2.txt"), "Contents of file2").unwrap();

    let output = run_cli(&["test_dir"], temp_dir.path());
    assert!(output.status.success());
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("test_dir/file1.txt"));
    assert!(stdout.contains("Contents of file1"));
    assert!(stdout.contains("test_dir/file2.txt"));
    assert!(stdout.contains("Contents of file2"));
}

#[test]
fn test_include_hidden() {
    let temp_dir = TempDir::new().unwrap();
    let test_dir = temp_dir.path().join("test_dir");
    fs::create_dir(&test_dir).unwrap();
    
    fs::write(test_dir.join(".hidden.txt"), "Contents of hidden file").unwrap();

    // Test without --include-hidden
    let output = run_cli(&["test_dir"], temp_dir.path());
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("test_dir/.hidden.txt"));

    // Test with --include-hidden
    let output = run_cli(&["test_dir", "--include-hidden"], temp_dir.path());
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("test_dir/.hidden.txt"));
    assert!(stdout.contains("Contents of hidden file"));
}

#[test]
fn test_ignore_gitignore() {
    let temp_dir = TempDir::new().unwrap();
    let test_dir = temp_dir.path().join("test_dir");
    fs::create_dir_all(&test_dir).unwrap();
    fs::create_dir_all(test_dir.join("nested_include")).unwrap();
    fs::create_dir_all(test_dir.join("nested_ignore")).unwrap();
    
    fs::write(test_dir.join(".gitignore"), "ignored.txt").unwrap();
    fs::write(test_dir.join("ignored.txt"), "This file should be ignored").unwrap();
    fs::write(test_dir.join("included.txt"), "This file should be included").unwrap();
    fs::write(test_dir.join("nested_include/included2.txt"), "This nested file should be included").unwrap();
    fs::write(test_dir.join("nested_ignore/.gitignore"), "nested_ignore.txt").unwrap();
    fs::write(test_dir.join("nested_ignore/nested_ignore.txt"), "This nested file should not be included").unwrap();
    fs::write(test_dir.join("nested_ignore/actually_include.txt"), "This nested file should actually be included").unwrap();

    // Test with gitignore respected (default)
    let output = run_cli(&["test_dir", "-c"], temp_dir.path());
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let filenames = filenames_from_cxml(&stdout);
    
    let expected: std::collections::HashSet<String> = [
        "test_dir/included.txt",
        "test_dir/nested_include/included2.txt",
        "test_dir/nested_ignore/actually_include.txt",
    ].iter().map(|s| s.to_string()).collect();
    
    assert_eq!(filenames, expected);

    // Test with --ignore-gitignore
    let output = run_cli(&["test_dir", "-c", "--ignore-gitignore"], temp_dir.path());
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let filenames = filenames_from_cxml(&stdout);
    
    let expected: std::collections::HashSet<String> = [
        "test_dir/included.txt",
        "test_dir/ignored.txt",
        "test_dir/nested_include/included2.txt",
        "test_dir/nested_ignore/nested_ignore.txt",
        "test_dir/nested_ignore/actually_include.txt",
    ].iter().map(|s| s.to_string()).collect();
    
    assert_eq!(filenames, expected);
}

#[test]
fn test_multiple_paths() {
    let temp_dir = TempDir::new().unwrap();
    let test_dir1 = temp_dir.path().join("test_dir1");
    let test_dir2 = temp_dir.path().join("test_dir2");
    fs::create_dir(&test_dir1).unwrap();
    fs::create_dir(&test_dir2).unwrap();
    
    fs::write(test_dir1.join("file1.txt"), "Contents of file1").unwrap();
    fs::write(test_dir2.join("file2.txt"), "Contents of file2").unwrap();
    fs::write(temp_dir.path().join("single_file.txt"), "Contents of single file").unwrap();

    let output = run_cli(&["test_dir1", "test_dir2", "single_file.txt"], temp_dir.path());
    assert!(output.status.success());
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("test_dir1/file1.txt"));
    assert!(stdout.contains("Contents of file1"));
    assert!(stdout.contains("test_dir2/file2.txt"));
    assert!(stdout.contains("Contents of file2"));
    assert!(stdout.contains("single_file.txt"));
    assert!(stdout.contains("Contents of single file"));
}

#[test]
fn test_ignore_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let test_dir = temp_dir.path().join("test_dir");
    fs::create_dir_all(&test_dir).unwrap();
    
    fs::write(test_dir.join("file_to_ignore.txt"), "This file should be ignored due to ignore patterns").unwrap();
    fs::write(test_dir.join("file_to_include.txt"), "This file should be included").unwrap();

    let output = run_cli(&["test_dir", "--ignore", "*.txt"], temp_dir.path());
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("test_dir/file_to_ignore.txt"));
    assert!(!stdout.contains("This file should be ignored due to ignore patterns"));
    assert!(!stdout.contains("test_dir/file_to_include.txt"));

    // Test subdirectory ignore patterns
    fs::create_dir_all(test_dir.join("test_subdir")).unwrap();
    fs::write(test_dir.join("test_subdir/any_file.txt"), "This entire subdirectory should be ignored due to ignore patterns").unwrap();
    
    let output = run_cli(&["test_dir", "--ignore", "*subdir*"], temp_dir.path());
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("test_dir/test_subdir/any_file.txt"));
    assert!(!stdout.contains("This entire subdirectory should be ignored due to ignore patterns"));
    assert!(stdout.contains("test_dir/file_to_include.txt"));
    assert!(stdout.contains("This file should be included"));

    // Test --ignore-files-only
    let output = run_cli(&["test_dir", "--ignore", "*subdir*", "--ignore-files-only"], temp_dir.path());
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("test_dir/test_subdir/any_file.txt"));
}

#[test]
fn test_specific_extensions() {
    let temp_dir = TempDir::new().unwrap();
    let test_dir = temp_dir.path().join("test_dir");
    fs::create_dir_all(test_dir.join("two")).unwrap();
    
    fs::write(test_dir.join("one.txt"), "This is one.txt").unwrap();
    fs::write(test_dir.join("one.py"), "This is one.py").unwrap();
    fs::write(test_dir.join("two/two.txt"), "This is two/two.txt").unwrap();
    fs::write(test_dir.join("two/two.py"), "This is two/two.py").unwrap();
    fs::write(test_dir.join("three.md"), "This is three.md").unwrap();

    // Try with -e py -e md
    let output = run_cli(&["test_dir", "-e", "py", "-e", "md"], temp_dir.path());
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains(".txt"));
    assert!(stdout.contains("test_dir/one.py"));
    assert!(stdout.contains("test_dir/two/two.py"));
    assert!(stdout.contains("test_dir/three.md"));
}

#[test]
fn test_mixed_paths_with_options() {
    let temp_dir = TempDir::new().unwrap();
    let test_dir = temp_dir.path().join("test_dir");
    fs::create_dir(&test_dir).unwrap();
    
    fs::write(test_dir.join(".gitignore"), "ignored_in_gitignore.txt\n.hidden_ignored_in_gitignore.txt").unwrap();
    fs::write(test_dir.join("ignored_in_gitignore.txt"), "This file should be ignored by .gitignore").unwrap();
    fs::write(test_dir.join(".hidden_ignored_in_gitignore.txt"), "This hidden file should be ignored by .gitignore").unwrap();
    fs::write(test_dir.join("included.txt"), "This file should be included").unwrap();
    fs::write(test_dir.join(".hidden_included.txt"), "This hidden file should be included").unwrap();
    fs::write(temp_dir.path().join("single_file.txt"), "Contents of single file").unwrap();

    // Test default behavior
    let output = run_cli(&["test_dir", "single_file.txt"], temp_dir.path());
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("test_dir/ignored_in_gitignore.txt"));
    assert!(!stdout.contains("test_dir/.hidden_ignored_in_gitignore.txt"));
    assert!(stdout.contains("test_dir/included.txt"));
    assert!(!stdout.contains("test_dir/.hidden_included.txt"));
    assert!(stdout.contains("single_file.txt"));
    assert!(stdout.contains("Contents of single file"));

    // Test with --include-hidden
    let output = run_cli(&["test_dir", "single_file.txt", "--include-hidden"], temp_dir.path());
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("test_dir/ignored_in_gitignore.txt"));
    assert!(!stdout.contains("test_dir/.hidden_ignored_in_gitignore.txt"));
    assert!(stdout.contains("test_dir/included.txt"));
    assert!(stdout.contains("test_dir/.hidden_included.txt"));
    assert!(stdout.contains("single_file.txt"));

    // Test with --ignore-gitignore
    let output = run_cli(&["test_dir", "single_file.txt", "--ignore-gitignore"], temp_dir.path());
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("test_dir/ignored_in_gitignore.txt"));
    assert!(!stdout.contains("test_dir/.hidden_ignored_in_gitignore.txt"));
    assert!(stdout.contains("test_dir/included.txt"));
    assert!(!stdout.contains("test_dir/.hidden_included.txt"));
    assert!(stdout.contains("single_file.txt"));

    // Test with both --ignore-gitignore and --include-hidden
    let output = run_cli(&["test_dir", "single_file.txt", "--ignore-gitignore", "--include-hidden"], temp_dir.path());
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("test_dir/ignored_in_gitignore.txt"));
    assert!(stdout.contains("test_dir/.hidden_ignored_in_gitignore.txt"));
    assert!(stdout.contains("test_dir/included.txt"));
    assert!(stdout.contains("test_dir/.hidden_included.txt"));
    assert!(stdout.contains("single_file.txt"));
}

#[test]
fn test_binary_file_warning() {
    let temp_dir = TempDir::new().unwrap();
    let test_dir = temp_dir.path().join("test_dir");
    fs::create_dir(&test_dir).unwrap();
    
    // Create binary file
    fs::write(test_dir.join("binary_file.bin"), &[0xff]).unwrap();
    fs::write(test_dir.join("text_file.txt"), "This is a text file").unwrap();

    let output = run_cli(&["test_dir"], temp_dir.path());
    assert!(output.status.success());
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    
    assert!(stdout.contains("test_dir/text_file.txt"));
    assert!(stdout.contains("This is a text file"));
    assert!(!stdout.contains("\ntest_dir/binary_file.bin"));
    assert!(stderr.contains("Warning: Skipping file test_dir/binary_file.bin due to UnicodeDecodeError"));
}

#[test]
fn test_xml_format_dir() {
    let temp_dir = TempDir::new().unwrap();
    let test_dir = temp_dir.path().join("test_dir");
    fs::create_dir(&test_dir).unwrap();
    
    fs::write(test_dir.join("file1.txt"), "Contents of file1.txt").unwrap();
    fs::write(test_dir.join("file2.txt"), "Contents of file2.txt").unwrap();

    let output = run_cli(&["test_dir", "--cxml"], temp_dir.path());
    assert!(output.status.success());
    
    let actual = String::from_utf8(output.stdout).unwrap();
    let expected = r#"
<documents>
<document index="1">
<source>test_dir/file1.txt</source>
<document_content>
Contents of file1.txt
</document_content>
</document>
<document index="2">
<source>test_dir/file2.txt</source>
<document_content>
Contents of file2.txt
</document_content>
</document>
</documents>
"#;
    assert_eq!(expected.trim(), actual.trim());

    // Test with individual files
    let output = run_cli(&["test_dir/file1.txt", "test_dir/file2.txt", "--cxml"], temp_dir.path());
    assert!(output.status.success());
    let actual = String::from_utf8(output.stdout).unwrap();
    assert_eq!(expected.trim(), actual.trim());
}

#[test]
fn test_output_option() {
    let temp_dir = TempDir::new().unwrap();
    let test_dir = temp_dir.path().join("test_dir");
    fs::create_dir(&test_dir).unwrap();
    
    fs::write(test_dir.join("file1.txt"), "Contents of file1.txt").unwrap();
    fs::write(test_dir.join("file2.txt"), "Contents of file2.txt").unwrap();

    let output_file = temp_dir.path().join("output.txt");
    
    // Test -o option
    let output = run_cli(&["test_dir", "-o", output_file.to_str().unwrap()], temp_dir.path());
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout).unwrap().is_empty());
    
    let actual = fs::read_to_string(&output_file).unwrap();
    let expected = r#"
test_dir/file1.txt
---
Contents of file1.txt

---
test_dir/file2.txt
---
Contents of file2.txt

---
"#;
    assert_eq!(expected.trim(), actual.trim());

    // Test --output option
    fs::remove_file(&output_file).unwrap();
    let output = run_cli(&["test_dir", "--output", output_file.to_str().unwrap()], temp_dir.path());
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout).unwrap().is_empty());
    
    let actual = fs::read_to_string(&output_file).unwrap();
    assert_eq!(expected.trim(), actual.trim());
}

#[test]
fn test_line_numbers() {
    let temp_dir = TempDir::new().unwrap();
    let test_dir = temp_dir.path().join("test_dir");
    fs::create_dir(&test_dir).unwrap();
    
    let test_content = "First line\nSecond line\nThird line\nFourth line\n";
    fs::write(test_dir.join("multiline.txt"), test_content).unwrap();

    // Test without line numbers
    let output = run_cli(&["test_dir"], temp_dir.path());
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("1  First line"));
    assert!(stdout.contains(test_content));

    // Test with -n option
    let output = run_cli(&["test_dir", "-n"], temp_dir.path());
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("1  First line"));
    assert!(stdout.contains("2  Second line"));
    assert!(stdout.contains("3  Third line"));
    assert!(stdout.contains("4  Fourth line"));

    // Test with --line-numbers option
    let output = run_cli(&["test_dir", "--line-numbers"], temp_dir.path());
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("1  First line"));
    assert!(stdout.contains("2  Second line"));
    assert!(stdout.contains("3  Third line"));
    assert!(stdout.contains("4  Fourth line"));
}

#[test]
fn test_reading_paths_from_stdin() {
    let temp_dir = TempDir::new().unwrap();
    fs::create_dir_all(temp_dir.path().join("test_dir1")).unwrap();
    fs::create_dir_all(temp_dir.path().join("test_dir2")).unwrap();
    
    fs::write(temp_dir.path().join("test_dir1/file1.txt"), "Contents of file1").unwrap();
    fs::write(temp_dir.path().join("test_dir2/file2.txt"), "Contents of file2").unwrap();

    // Test newline-separated paths from stdin
    let input = "test_dir1/file1.txt\ntest_dir2/file2.txt";
    let output = run_cli_with_stdin(&[], temp_dir.path(), input);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("test_dir1/file1.txt"));
    assert!(stdout.contains("Contents of file1"));
    assert!(stdout.contains("test_dir2/file2.txt"));
    assert!(stdout.contains("Contents of file2"));

    // Test null-separated paths from stdin
    let input = "test_dir1/file1.txt\0test_dir2/file2.txt";
    let output = run_cli_with_stdin(&["--null"], temp_dir.path(), input);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("test_dir1/file1.txt"));
    assert!(stdout.contains("Contents of file1"));
    assert!(stdout.contains("test_dir2/file2.txt"));
    assert!(stdout.contains("Contents of file2"));

    // Test -0 option
    let output = run_cli_with_stdin(&["-0"], temp_dir.path(), input);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("test_dir1/file1.txt"));
    assert!(stdout.contains("Contents of file1"));
    assert!(stdout.contains("test_dir2/file2.txt"));
    assert!(stdout.contains("Contents of file2"));
}

#[test]
fn test_paths_from_arguments_and_stdin() {
    let temp_dir = TempDir::new().unwrap();
    fs::create_dir_all(temp_dir.path().join("test_dir1")).unwrap();
    fs::create_dir_all(temp_dir.path().join("test_dir2")).unwrap();
    
    fs::write(temp_dir.path().join("test_dir1/file1.txt"), "Contents of file1").unwrap();
    fs::write(temp_dir.path().join("test_dir2/file2.txt"), "Contents of file2").unwrap();

    // Test paths from arguments and stdin
    let input = "test_dir2/file2.txt";
    let output = run_cli_with_stdin(&["test_dir1"], temp_dir.path(), input);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("test_dir1/file1.txt"));
    assert!(stdout.contains("Contents of file1"));
    assert!(stdout.contains("test_dir2/file2.txt"));
    assert!(stdout.contains("Contents of file2"));
}

#[test]
fn test_markdown() {
    let temp_dir = TempDir::new().unwrap();
    let test_dir = temp_dir.path().join("test_dir");
    fs::create_dir(&test_dir).unwrap();
    
    fs::write(test_dir.join("python.py"), "This is python").unwrap();
    fs::write(test_dir.join("python_with_quad_backticks.py"), "This is python with ```` in it already").unwrap();
    fs::write(test_dir.join("code.js"), "This is javascript").unwrap();
    fs::write(test_dir.join("code.unknown"), "This is an unknown file type").unwrap();

    // Test -m option
    let output = run_cli(&["test_dir", "-m"], temp_dir.path());
    assert!(output.status.success());
    let actual = String::from_utf8(output.stdout).unwrap();
    
    let expected = r#"test_dir/code.js
```javascript
This is javascript
```
test_dir/code.unknown
```
This is an unknown file type
```
test_dir/python.py
```python
This is python
```
test_dir/python_with_quad_backticks.py
`````python
This is python with ```` in it already
`````"#;
    
    assert_eq!(expected.trim(), actual.trim());

    // Test --markdown option
    let output = run_cli(&["test_dir", "--markdown"], temp_dir.path());
    assert!(output.status.success());
    let actual = String::from_utf8(output.stdout).unwrap();
    assert_eq!(expected.trim(), actual.trim());
}

#[cfg(test)]
mod tests {
    use super::*;
}