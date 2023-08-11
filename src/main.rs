use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone)]
struct Config {
    pattern: String,
    path: String,
    recursive: bool,
    num_threads: usize,
}

fn main() {
    // Parse command-line arguments and obtain a Config struct
    let config = parse_args(env::args()).unwrap_or_else(|err| {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    });

    // Create a shared result container using Arc and Mutex
    let results = Arc::new(Mutex::new(Vec::new()));

    // Collect file paths based on the configuration
    let mut file_paths = Vec::new();
    if config.recursive {
        walk_dir(&config.path, &mut file_paths);
    } else {
        file_paths.push(PathBuf::from(&config.path));
    }

    // Create threads to search the file paths in parallel
    let mut threads = Vec::new();
    let chunk_size = file_paths.len() / config.num_threads;

    for i in 0..config.num_threads {
        let start = i * chunk_size;
        let end = if i == config.num_threads - 1 {
            file_paths.len()
        } else {
            (i + 1) * chunk_size
        };

        let results = Arc::clone(&results);
        let config = config.clone();
        let files = file_paths[start..end].to_vec();

        threads.push(thread::spawn(move || {
            for path in files {
                if path.is_file() {
                    search_file(&path, &config, &results);
                }
            }
        }));
    }

    // Wait for all threads to finish
    for thread in threads {
        thread.join().unwrap();
    }

    // Print the search results
    let results = results.lock().unwrap();
    for result in results.iter() {
        println!("{}", result);
    }
}

// Search a file for lines matching the pattern
fn search_file(path: &PathBuf, config: &Config, results: &Arc<Mutex<Vec<String>>>) {
    // Open the file and read its lines
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return,
    };

    let reader = BufReader::new(file);

    for (i, line) in reader.lines().enumerate() {
        let line = line.unwrap();
        let match_result = match_line_number(&line, &config.pattern, i + 1);

        // Add the matching line to the results
        if match_result {
            results
                .lock()
                .unwrap()
                .push(format!("{}:{}", path.display(), line));
        }
    }
}

// Match a line against a pattern and print it with the line number
fn match_line_number(line: &str, pattern: &str, line_number: usize) -> bool {
    let match_result = line.contains(pattern);

    if match_result {
        println!("{}:{}", line_number, line);
    }

    match_result
}

// Recursively walk a directory and collect file paths
fn walk_dir(path: &str, files: &mut Vec<PathBuf>) {
    let path = PathBuf::from(path);

    if path.is_file() {
        files.push(path);
    } else if path.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let child_path = entry.path().to_str().unwrap().to_owned();

            if entry.path().is_file() {
                files.push(PathBuf::from(child_path));
            } else if entry.path().is_dir() {
                walk_dir(&child_path, files);
            }
        }
    }
}

// Parse command-line arguments and construct a Config struct
fn parse_args(mut args: env::Args) -> Result<Config, String> {
    args.next(); // skip program name

    let mut pattern = String::new();
    let mut path = String::new();
    let mut recursive = false;
    let mut num_threads = 2;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-r" | "--recursive" => recursive = true,
            "-n" | "--num-threads" => {
                num_threads = args
                    .next()
                    .ok_or("Missing number of threads".to_owned())?
                    .parse::<usize>()
                    .map_err(|_| "Number of threads must be an integer".to_owned())?;
            }
            _ if pattern.is_empty() => pattern = arg,
            _ if path.is_empty() => path = arg,
            _ => return Err(format!("Unknown argument: {}", arg)),
        }
    }

    if pattern.is_empty() {
        return Err("Missing pattern argument".to_owned());
    }

    if path.is_empty() {
        return Err("Missing path argument".to_owned());
    }

    Ok(Config {
        pattern,
        path,
        recursive,
        num_threads,
    })
}
