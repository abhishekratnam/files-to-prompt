use clap::{arg, command, ArgAction}; // Uncomment and remove Command
use glob::Pattern;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static GLOBAL_INDEX: AtomicUsize = AtomicUsize::new(1);


pub fn run() -> io::Result<()> {
    // Fix the -0 flag by using the more verbose Arg construction instead of arg! macro
    let matches = command!()
        .about("Concatenate a directory full of files into a single prompt for use with LLMs")
        .arg(arg!([PATHS] ... "Paths to files or directories").required(false))
        .arg(arg!(-e --extension <EXT> ... "File extensions to include"))
        .arg(arg!(--"include-hidden" "Include files and folders starting with .").action(ArgAction::SetTrue))
        .arg(arg!(--"ignore-files-only" "--ignore option only ignores files").action(ArgAction::SetTrue))
        .arg(arg!(--"ignore-gitignore" "Ignore .gitignore files and include all files").action(ArgAction::SetTrue))
        .arg(arg!(--ignore <PATTERN> ... "List of patterns to ignore"))
        .arg(arg!(-o --output <FILE> "Output to a file instead of stdout"))
        .arg(arg!(-c --cxml "Output in XML-ish format suitable for Claude's long context window").action(ArgAction::SetTrue))
        .arg(arg!(-m --markdown "Output Markdown with fenced code blocks").action(ArgAction::SetTrue))
        .arg(arg!(-n --"line-numbers" "Add line numbers to the output").action(ArgAction::SetTrue))
        // Replace this with a properly constructed Arg
        .arg(
            clap::Arg::new("null")
                .short('0') // This works with the numeric 0
                .long("null")
                .help("Use NUL character as separator when reading from stdin")
                .action(ArgAction::SetTrue)
        )
        .get_matches();

    // Initialize the extension to language mapping
    let ext_to_lang = initialize_ext_to_lang();

    // Reset global index
    GLOBAL_INDEX.store(1, Ordering::SeqCst);

    // Get paths from CLI args
    let mut paths: Vec<PathBuf> = matches
        .get_many::<String>("PATHS")
        .unwrap_or_default()
        .map(|s| PathBuf::from(s))
        .collect();

    // Read paths from stdin if available
    let use_null_separator = matches.get_flag("null");
    let stdin_paths = read_paths_from_stdin(use_null_separator)?;
    paths.extend(stdin_paths);

    // Setup output writer
    let mut output_file: Option<File> = None;
    if let Some(output_path) = matches.get_one::<String>("output") {
        output_file = Some(File::create(output_path)?);
    }

    let claude_xml = matches.get_flag("cxml");
    let markdown = matches.get_flag("markdown");
    let line_numbers = matches.get_flag("line-numbers");
    let include_hidden = matches.get_flag("include-hidden");
    let ignore_files_only = matches.get_flag("ignore-files-only");
    let ignore_gitignore = matches.get_flag("ignore-gitignore");
    
    let extensions: Vec<String> = matches
        .get_many::<String>("extension")
        .unwrap_or_default()
        .cloned()
        .collect();
    
    let ignore_patterns: Vec<String> = matches
        .get_many::<String>("ignore")
        .unwrap_or_default()
        .cloned()
        .collect();

    let mut gitignore_rules = Vec::new();

    // Start XML document if needed
    if claude_xml && !paths.is_empty() {
        write_output("<documents>", &mut output_file)?;
    }

    // Process each path
    for path in &paths {
        if !path.exists() {
            eprintln!("Path does not exist: {}", path.display());
            continue;
        }

        if !ignore_gitignore {
            if let Some(parent) = path.parent() {
                gitignore_rules.extend(read_gitignore(parent)?);
            }
        }

        process_path(
            path,
            &extensions,
            include_hidden,
            ignore_files_only,
            ignore_gitignore,
            &mut gitignore_rules,
            &ignore_patterns,
            &mut output_file,
            claude_xml,
            markdown,
            line_numbers,
            &ext_to_lang,
        )?;
    }

    // Close XML document if needed
    if claude_xml {
        write_output("</documents>", &mut output_file)?;
    }

    Ok(())
}

fn initialize_ext_to_lang() -> HashMap<String, &'static str> {
    let mut map = HashMap::new();
    map.insert("py".to_string(), "python");
    map.insert("c".to_string(), "c");
    map.insert("cpp".to_string(), "cpp");
    map.insert("java".to_string(), "java");
    map.insert("js".to_string(), "javascript");
    map.insert("ts".to_string(), "typescript");
    map.insert("html".to_string(), "html");
    map.insert("css".to_string(), "css");
    map.insert("xml".to_string(), "xml");
    map.insert("json".to_string(), "json");
    map.insert("yaml".to_string(), "yaml");
    map.insert("yml".to_string(), "yaml");
    map.insert("sh".to_string(), "bash");
    map.insert("rb".to_string(), "ruby");
    map
}

fn should_ignore(path: &Path, gitignore_rules: &[String]) -> bool {
    let basename = path.file_name().unwrap_or_default().to_string_lossy();
    
    for rule in gitignore_rules {
        let pattern = Pattern::new(rule).unwrap_or_else(|_| Pattern::new("*").unwrap());
        
        if pattern.matches(&basename) {
            return true;
        }
        
        if path.is_dir() && pattern.matches(&format!("{}/", basename)) {
            return true;
        }
    }
    
    false
}

fn read_gitignore(path: &Path) -> io::Result<Vec<String>> {
    let gitignore_path = path.join(".gitignore");
    
    if !gitignore_path.is_file() {
        return Ok(Vec::new());
    }
    
    let file = File::open(gitignore_path)?;
    let reader = io::BufReader::new(file);
    let mut rules = Vec::new();
    
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            rules.push(trimmed.to_string());
        }
    }
    
    Ok(rules)
}

fn add_line_numbers(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let padding = lines.len().to_string().len();
    
    lines
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:width$}  {}", i + 1, line, width = padding))
        .collect::<Vec<String>>()
        .join("\n")
}

fn print_path(
    path: &Path,
    content: &str,
    output_file: &mut Option<File>,
    cxml: bool,
    markdown: bool,
    line_numbers: bool,
    ext_to_lang: &HashMap<String, &'static str>,
) -> io::Result<()> {
    if cxml {
        print_as_xml(path, content, output_file, line_numbers)
    } else if markdown {
        print_as_markdown(path, content, output_file, line_numbers, ext_to_lang)
    } else {
        print_default(path, content, output_file, line_numbers)
    }
}

fn print_default(
    path: &Path,
    content: &str,
    output_file: &mut Option<File>,
    line_numbers: bool,
) -> io::Result<()> {
    write_output(&path.display().to_string(), output_file)?;
    write_output("---", output_file)?;
    
    let output_content = if line_numbers {
        add_line_numbers(content)
    } else {
        content.to_string()
    };
    
    write_output(&output_content, output_file)?;
    write_output("", output_file)?;
    write_output("---", output_file)?;
    
    Ok(())
}

fn print_as_xml(
    path: &Path,
    content: &str,
    output_file: &mut Option<File>,
    line_numbers: bool,
) -> io::Result<()> {
    let index = GLOBAL_INDEX.fetch_add(1, Ordering::SeqCst);
    
    write_output(&format!("<document index=\"{}\">", index), output_file)?;
    write_output(&format!("<source>{}</source>", path.display()), output_file)?;
    write_output("<document_content>", output_file)?;
    
    let output_content = if line_numbers {
        add_line_numbers(content)
    } else {
        content.to_string()
    };
    
    write_output(&output_content, output_file)?;
    write_output("</document_content>", output_file)?;
    write_output("</document>", output_file)?;
    
    Ok(())
}

fn print_as_markdown(
    path: &Path,
    content: &str,
    output_file: &mut Option<File>,
    line_numbers: bool,
    ext_to_lang: &HashMap<String, &'static str>,
) -> io::Result<()> {
    let extension = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    
    let lang = ext_to_lang.get(&extension).unwrap_or(&"");
    
    // Determine number of backticks needed
    let mut backticks = "```".to_string();
    while content.contains(&backticks) {
        backticks.push('`');
    }
    
    write_output(&path.display().to_string(), output_file)?;
    write_output(&format!("{}{}", backticks, lang), output_file)?;
    
    let output_content = if line_numbers {
        add_line_numbers(content)
    } else {
        content.to_string()
    };
    
    write_output(&output_content, output_file)?;
    write_output(&backticks, output_file)?;
    
    Ok(())
}

fn process_path(
    path: &Path,
    extensions: &[String],
    include_hidden: bool,
    ignore_files_only: bool,
    ignore_gitignore: bool,
    gitignore_rules: &mut Vec<String>,
    ignore_patterns: &[String],
    output_file: &mut Option<File>,
    claude_xml: bool,
    markdown: bool,
    line_numbers: bool,
    ext_to_lang: &HashMap<String, &'static str>,
) -> io::Result<()> {
    if path.is_file() {
        match fs::read_to_string(path) {
            Ok(content) => {
                print_path(
                    path,
                    &content,
                    output_file,
                    claude_xml,
                    markdown,
                    line_numbers,
                    ext_to_lang,
                )?;
            }
            Err(e) => {
                eprintln!("Warning: Skipping file {} due to error: {}", path.display(), e);
            }
        }
    } else if path.is_dir() {
        walk_directory(
            path,
            extensions,
            include_hidden,
            ignore_files_only,
            ignore_gitignore,
            gitignore_rules,
            ignore_patterns,
            output_file,
            claude_xml,
            markdown,
            line_numbers,
            ext_to_lang,
        )?;
    }
    
    Ok(())
}

fn walk_directory(
    dir: &Path,
    extensions: &[String],
    include_hidden: bool,
    ignore_files_only: bool,
    ignore_gitignore: bool,
    gitignore_rules: &mut Vec<String>,
    ignore_patterns: &[String],
    output_file: &mut Option<File>,
    claude_xml: bool,
    markdown: bool,
    line_numbers: bool,
    ext_to_lang: &HashMap<String, &'static str>,
) -> io::Result<()> {
    if !ignore_gitignore {
        gitignore_rules.extend(read_gitignore(dir)?);
    }
    
    let mut entries: Vec<fs::DirEntry> = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|entry| {
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            
            // Check if hidden
            if !include_hidden && name_str.starts_with('.') {
                return false;
            }
            
            // Check gitignore rules
            if !ignore_gitignore && should_ignore(&path, gitignore_rules) {
                return false;
            }
            
            // Check ignore patterns
            if !ignore_patterns.is_empty() {
                let is_dir = path.is_dir();
                if !is_dir || !ignore_files_only {
                    for pattern in ignore_patterns {
                        let fnpattern = Pattern::new(pattern).unwrap_or_else(|_| Pattern::new("*").unwrap());
                        if fnpattern.matches(&name_str) {
                            return false;
                        }
                    }
                }
            }
            
            true
        })
        .collect();
    
    // Sort entries by name
    entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    
    for entry in entries {
        let path = entry.path();
        
        if path.is_dir() {
            walk_directory(
                &path,
                extensions,
                include_hidden,
                ignore_files_only,
                ignore_gitignore,
                gitignore_rules,
                ignore_patterns,
                output_file,
                claude_xml,
                markdown,
                line_numbers,
                ext_to_lang,
            )?;
        } else if path.is_file() {
            // Check extensions
            if !extensions.is_empty() {
                let ext = path
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if !extensions.iter().any(|e| *e == ext) {
                    continue;
                }
            }
            
            match fs::read_to_string(&path) {
                Ok(content) => {
                    print_path(
                        &path,
                        &content,
                        output_file,
                        claude_xml,
                        markdown,
                        line_numbers,
                        ext_to_lang,
                    )?;
                }
                Err(e) => {
                    eprintln!("Warning: Skipping file {} due to error: {}", path.display(), e);
                }
            }
        }
    }
    
    Ok(())
}

fn read_paths_from_stdin(use_null_separator: bool) -> io::Result<Vec<PathBuf>> {
    let stdin = io::stdin();
    
    // Check if stdin is a TTY (interactive terminal)
    if atty::is(atty::Stream::Stdin) {
        return Ok(Vec::new());
    }
    
    let mut content = String::new();
    stdin.lock().read_to_string(&mut content)?;
    
    let paths = if use_null_separator {
        content
            .split('\0')
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .collect()
    } else {
        content
            .split_whitespace()
            .map(PathBuf::from)
            .collect()
    };
    
    Ok(paths)
}

fn write_output(content: &str, output_file: &mut Option<File>) -> io::Result<()> {
    match output_file {
        Some(file) => {
            writeln!(file, "{}", content)?;
        }
        None => {
            println!("{}", content);
        }
    }
    Ok(())
}
