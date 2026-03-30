import Foundation

enum Command: String {
    case extract
    case benchmark
    case inspectProxies = "inspect-proxies"
    case spec
    case help
}

func printUsage() {
    let text = """
    lifelog-compression

    A macOS-native utility for turning source video into sparse visual bundles
    before upload.

    Current intended commands:

      extract
        Extract a visual bundle from source video.
        Target default:
          - 1 fps
          - padded 1920x1080 JPEG
          - +/- 0.5s tolerance

      benchmark
        Run local preprocessing benchmarks on real video inputs.

      inspect-proxies
        Inspect a source directory for likely proxy / preview companions.

      spec
        Print the current format and design pointers.

    This package is currently an initial scaffold. See:
      - docs/visual-bundle-v1.md
      - docs/technical-findings.md
      - docs/benchmark-2026-03-30-real-workload.md
    """
    print(text)
}

func printSpecPointer() {
    let text = """
    Current visual-bundle v1 direction:

    - canonical representation: JPEG frames + JSONL manifest
    - canonical canvas: padded 1920x1080
    - cadence: 1 fps
    - timestamp tolerance: +/- 0.5s
    - upload transport: tar shards

    See docs/visual-bundle-v1.md for the full spec.
    """
    print(text)
}

let args = Array(CommandLine.arguments.dropFirst())
let command = args.first.flatMap(Command.init(rawValue:)) ?? .help

switch command {
case .extract:
    fputs("extract is not implemented yet. See docs/visual-bundle-v1.md for the intended output shape.\n", stderr)
    exit(2)
case .benchmark:
    fputs("benchmark is not implemented yet. See docs/benchmark-2026-03-30-real-workload.md for current results.\n", stderr)
    exit(2)
case .inspectProxies:
    fputs("inspect-proxies is not implemented yet.\n", stderr)
    exit(2)
case .spec:
    printSpecPointer()
case .help:
    printUsage()
}
