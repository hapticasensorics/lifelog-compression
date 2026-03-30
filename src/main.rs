use lifelog_compression::{ExtractDefaults, ExtractRequest, extract, extract_directory_to_dir};
use std::env;
use std::path::PathBuf;

fn print_usage() {
    println!(
        r#"lifelog-compression

An opinionated utility for turning one source video into one sparse visual bundle.

Commands:
  extract <input-video> <output-dir>
    Intended default:
      - 1 fps
      - padded 1920x1080 JPEG
      - +/- 0.5s tolerance
      - preserve intrinsic source-video metadata
      - one input video -> one output bundle

  batch <input-dir> <output-root>
    Recursively extract every supported video under a directory.
    Output bundle folders mirror the relative input paths.

  spec
    Print the current bundle design summary.

  help
    Show this message.
"#
    );
}

fn print_spec() {
    let defaults = ExtractDefaults::default();
    println!(
        r#"Current visual-bundle v1 direction:

- canonical representation: JPEG frames + JSONL manifest
- canonical canvas: padded {w}x{h}
- cadence: {fps_num}/{fps_den} fps
- timestamp tolerance: -{tb}ms / +{ta}ms
- preserve intrinsic source-video metadata
- one input video -> one bundle -> one upload unit
- optional packaging: single tar archive of the bundle

See:
  - docs/visual-bundle-v1.md
  - docs/technical-findings.md
  - docs/benchmark-2026-03-30-real-workload.md
"#,
        w = defaults.canvas_width,
        h = defaults.canvas_height,
        fps_num = defaults.target_fps_numerator,
        fps_den = defaults.target_fps_denominator,
        tb = defaults.tolerance_before_ms,
        ta = defaults.tolerance_after_ms,
    );
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some(command) = args.first().map(String::as_str) else {
        print_usage();
        return;
    };

    match command {
        "help" | "--help" | "-h" => print_usage(),
        "spec" => print_spec(),
        "extract" => {
            if args.len() != 3 {
                eprintln!("usage: lifelog-compression extract <input-video> <output-dir>");
                std::process::exit(2);
            }
            let request = ExtractRequest::new(PathBuf::from(&args[1]), PathBuf::from(&args[2]));
            match extract(&request) {
                Ok(result) => {
                    println!("{}", result.bundle_dir.display());
                }
                Err(message) => {
                    eprintln!("{message}");
                    std::process::exit(2);
                }
            }
        }
        "batch" => {
            if args.len() != 3 {
                eprintln!("usage: lifelog-compression batch <input-dir> <output-root>");
                std::process::exit(2);
            }
            match extract_directory_to_dir(PathBuf::from(&args[1]), PathBuf::from(&args[2])) {
                Ok(result) => {
                    println!(
                        "{}\nitems={}\nframes={}",
                        result.output_root.display(),
                        result.item_count,
                        result.total_frame_count
                    );
                }
                Err(message) => {
                    eprintln!("{message}");
                    std::process::exit(2);
                }
            }
        }
        _ => {
            eprintln!("unknown command: {command}");
            print_usage();
            std::process::exit(2);
        }
    }
}
